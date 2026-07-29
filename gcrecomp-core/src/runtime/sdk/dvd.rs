//! Virtual DVD filesystem for GameCube disc asset access.
//!
//! Parses a GCFS archive (built by `disc_fs::build_archive`) and provides
//! `DVDOpen` / `DVDRead` / `DVDClose` / `DVDGetLength` emulation so
//! recompiled games can load assets at runtime.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

use crate::runtime::memory::MemoryManager;

/// Table-of-contents entry parsed from the GCFS archive.
struct TocEntry {
    /// Byte offset of compressed data within the archive.
    data_offset: usize,
    /// Size of the zstd-compressed data.
    compressed_size: usize,
    /// Size after decompression.
    decompressed_size: usize,
}

/// State for a currently open file handle.
struct OpenFile {
    path: String,
    length: u32,
    override_path: Option<PathBuf>,
}

enum ArchiveSource {
    Embedded(&'static [u8]),
    File { path: PathBuf, len: usize },
}

impl ArchiveSource {
    fn len(&self) -> usize {
        match self {
            Self::Embedded(data) => data.len(),
            Self::File { len, .. } => *len,
        }
    }

    fn read_range(&self, offset: usize, length: usize) -> Result<Vec<u8>, String> {
        let end = offset
            .checked_add(length)
            .ok_or_else(|| "GCFS archive range overflow".to_string())?;
        if end > self.len() {
            return Err(format!(
                "GCFS archive range {}..{} exceeds {} bytes",
                offset,
                end,
                self.len()
            ));
        }
        match self {
            Self::Embedded(data) => Ok(data[offset..end].to_vec()),
            Self::File { path, .. } => {
                let mut file = std::fs::File::open(path)
                    .map_err(|error| format!("opening {}: {error}", path.display()))?;
                file.seek(SeekFrom::Start(offset as u64))
                    .map_err(|error| format!("seeking {}: {error}", path.display()))?;
                let mut data = vec![0u8; length];
                file.read_exact(&mut data)
                    .map_err(|error| format!("reading {}: {error}", path.display()))?;
                Ok(data)
            }
        }
    }
}

/// Virtual filesystem backed by an embedded or external GCFS archive.
pub struct VirtualFilesystem {
    archive: ArchiveSource,
    /// Path → TOC entry mapping.
    toc: HashMap<String, TocEntry>,
    /// Lazily decompressed file cache.
    file_cache: HashMap<String, Vec<u8>>,
    /// Open file handles: handle_id → OpenFile.
    open_files: HashMap<u32, OpenFile>,
    /// Next handle ID to assign (starts at 1; 0 means failure).
    next_handle: u32,
    /// Optional loose-file tree. Files here override matching disc files.
    override_dir: Option<PathBuf>,
}

impl VirtualFilesystem {
    /// Parse a GCFS archive and build the TOC index.
    ///
    /// GCFS header layout (all little-endian):
    /// ```text
    /// [0..4]   magic b"GCFS"
    /// [4..8]   version u32
    /// [8..12]  file_count u32
    /// [12..20] toc_offset u64
    /// ```
    pub fn new(archive: &'static [u8]) -> Result<Self, String> {
        if archive.is_empty() {
            return Ok(Self {
                archive: ArchiveSource::Embedded(archive),
                toc: HashMap::new(),
                file_cache: HashMap::new(),
                open_files: HashMap::new(),
                next_handle: 1,
                override_dir: override_directory(),
            });
        }

        Self::from_source(ArchiveSource::Embedded(archive))
    }

    /// Open a GCFS archive without embedding its potentially multi-gigabyte
    /// contents in the executable.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let len = usize::try_from(
            std::fs::metadata(&path)
                .map_err(|error| format!("reading {} metadata: {error}", path.display()))?
                .len(),
        )
        .map_err(|_| format!("archive {} is too large for this host", path.display()))?;
        Self::from_source(ArchiveSource::File { path, len })
    }

    fn from_source(archive: ArchiveSource) -> Result<Self, String> {
        if archive.len() < 20 {
            return Err("GCFS archive too small for header.".to_string());
        }

        let header = archive.read_range(0, 20)?;
        if &header[0..4] != b"GCFS" {
            return Err("Invalid GCFS magic.".to_string());
        }

        let _version = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
        let file_count =
            u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize;
        let toc_offset = read_u64_le(&header, 12) as usize;

        if toc_offset > archive.len() {
            return Err(format!(
                "GCFS TOC offset {} exceeds archive size {}.",
                toc_offset,
                archive.len()
            ));
        }

        let toc_data = archive.read_range(toc_offset, archive.len() - toc_offset)?;
        let mut toc = HashMap::with_capacity(file_count);
        let mut pos = 0;

        for _ in 0..file_count {
            if pos + 2 > toc_data.len() {
                return Err("GCFS TOC truncated (path_len).".to_string());
            }
            let path_len = u16::from_le_bytes([toc_data[pos], toc_data[pos + 1]]) as usize;
            pos += 2;

            if pos + path_len > toc_data.len() {
                return Err("GCFS TOC truncated (path).".to_string());
            }
            let path = String::from_utf8_lossy(&toc_data[pos..pos + path_len]).into_owned();
            pos += path_len;

            if pos + 24 > toc_data.len() {
                return Err("GCFS TOC truncated (offsets).".to_string());
            }
            let data_offset = read_u64_le(&toc_data, pos) as usize;
            pos += 8;
            let compressed_size = read_u64_le(&toc_data, pos) as usize;
            pos += 8;
            let decompressed_size = read_u64_le(&toc_data, pos) as usize;
            pos += 8;

            toc.insert(
                path,
                TocEntry {
                    data_offset,
                    compressed_size,
                    decompressed_size,
                },
            );
        }

        log::info!(
            "DVD filesystem initialized: loaded {} files from GCFS archive.",
            toc.len()
        );

        Ok(Self {
            archive,
            toc,
            file_cache: HashMap::new(),
            open_files: HashMap::new(),
            next_handle: 1,
            override_dir: override_directory(),
        })
    }

    /// Open a file by path. Returns a handle (>0) or 0 on failure.
    ///
    /// GameCube games use paths like `/banner.bnr` or `audio/stream.adp`.
    /// We normalize by stripping a leading `/` if present.
    pub fn dvd_open(&mut self, path: &str) -> u32 {
        let normalized = path.strip_prefix('/').unwrap_or(path);

        if let Some(override_path) = self
            .override_dir
            .as_ref()
            .and_then(|root| safe_join(root, normalized))
            .filter(|candidate| candidate.is_file())
        {
            match std::fs::metadata(&override_path) {
                Ok(metadata) => {
                    let handle = self.next_handle;
                    self.next_handle += 1;
                    self.open_files.insert(
                        handle,
                        OpenFile {
                            path: normalized.replace('\\', "/"),
                            length: metadata.len().min(u32::MAX as u64) as u32,
                            override_path: Some(override_path.clone()),
                        },
                    );
                    log::info!(
                        "DVDOpen('{}') -> loose mod {}",
                        path,
                        override_path.display()
                    );
                    return handle;
                }
                Err(error) => {
                    log::warn!("Could not inspect loose mod '{}': {error}", path);
                }
            }
        }

        // Try exact match first, then case-insensitive
        let found = if self.toc.contains_key(normalized) {
            Some(normalized.to_string())
        } else {
            let lower = normalized.to_lowercase();
            self.toc.keys().find(|k| k.to_lowercase() == lower).cloned()
        };

        match found {
            Some(key) => {
                let entry = &self.toc[&key];
                let length = entry.decompressed_size as u32;
                let handle = self.next_handle;
                self.next_handle += 1;
                self.open_files.insert(
                    handle,
                    OpenFile {
                        path: key,
                        length,
                        override_path: None,
                    },
                );
                log::debug!("DVDOpen('{}') -> handle {}", path, handle);
                handle
            }
            None => {
                log::warn!("DVDOpen('{}') -> file not found", path);
                0
            }
        }
    }

    /// Close a file handle. Returns true if the handle was valid.
    pub fn dvd_close(&mut self, handle: u32) -> bool {
        let removed = self.open_files.remove(&handle).is_some();
        if removed {
            log::debug!("DVDClose(handle={}) -> ok", handle);
        } else {
            log::warn!("DVDClose(handle={}) -> invalid handle", handle);
        }
        removed
    }

    /// Get the decompressed file length for an open handle.
    pub fn dvd_get_length(&self, handle: u32) -> u32 {
        self.open_files.get(&handle).map(|f| f.length).unwrap_or(0)
    }

    /// Read data from an open file into GameCube memory.
    ///
    /// Decompresses the file on first access and caches the result.
    /// Copies `length` bytes starting at `offset` in the file to `gc_addr` in memory.
    /// Returns the number of bytes actually read.
    pub fn dvd_read(
        &mut self,
        handle: u32,
        memory: &mut MemoryManager,
        gc_addr: u32,
        length: u32,
        offset: u32,
    ) -> Result<u32, String> {
        let file_info = self
            .open_files
            .get(&handle)
            .ok_or_else(|| format!("DVDRead: invalid handle {}", handle))?;

        let path = file_info.path.clone();
        if let Some(override_path) = file_info.override_path.as_ref() {
            let file_data = std::fs::read(override_path).map_err(|error| {
                format!(
                    "DVDRead: failed to read loose mod '{}': {error}",
                    override_path.display()
                )
            })?;
            return copy_file_range(&file_data, memory, gc_addr, length, offset);
        }

        // Decompress on first access
        if !self.file_cache.contains_key(&path) {
            let toc_entry = self
                .toc
                .get(&path)
                .ok_or_else(|| format!("DVDRead: TOC entry missing for '{}'", path))?;

            let compressed_end = toc_entry.data_offset + toc_entry.compressed_size;
            if compressed_end > self.archive.len() {
                return Err(format!(
                    "DVDRead: compressed data for '{}' out of bounds.",
                    path
                ));
            }

            let compressed = self
                .archive
                .read_range(toc_entry.data_offset, toc_entry.compressed_size)?;
            let decompressed = zstd::decode_all(compressed.as_slice())
                .map_err(|e| format!("DVDRead: zstd decompression failed for '{}': {}", path, e))?;

            log::debug!(
                "DVDRead: decompressed '{}' ({} -> {} bytes)",
                path,
                toc_entry.compressed_size,
                decompressed.len()
            );
            self.file_cache.insert(path.clone(), decompressed);
        }

        copy_file_range(&self.file_cache[&path], memory, gc_addr, length, offset)
    }
}

fn override_directory() -> Option<PathBuf> {
    let path = std::env::var_os("GCRECOMP_MOD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("mods").join("files"));
    if path.is_dir() {
        log::info!("Loose disc-file mods enabled from {}", path.display());
        Some(path)
    } else {
        None
    }
}

fn safe_join(root: &Path, relative: &str) -> Option<PathBuf> {
    let mut output = root.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => output.push(part),
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => return None,
        }
    }
    Some(output)
}

fn copy_file_range(
    file_data: &[u8],
    memory: &mut MemoryManager,
    gc_addr: u32,
    length: u32,
    offset: u32,
) -> Result<u32, String> {
    let start = offset as usize;
    if start >= file_data.len() {
        return Ok(0);
    }
    let end = start.saturating_add(length as usize).min(file_data.len());
    let slice = &file_data[start..end];
    memory
        .write_bytes(gc_addr, slice)
        .map_err(|error| format!("DVDRead: memory write failed at 0x{gc_addr:08X}: {error}"))?;
    Ok(slice.len() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loose_mod_paths_cannot_escape_their_root() {
        let root = Path::new("mods/files");
        assert!(safe_join(root, "scene/map.bmd").is_some());
        assert!(safe_join(root, "../private.bin").is_none());
        assert!(safe_join(root, "/absolute.bin").is_none());
    }
}

fn read_u64_le(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ])
}

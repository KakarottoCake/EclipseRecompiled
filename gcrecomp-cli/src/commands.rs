// CLI command handlers
use anyhow::{Context, Result};
use gcrecomp_core::recompiler::{parser::DolFile, pipeline::RecompilationPipeline};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub fn analyze_dol(dol_file: &Path, symbol_map: Option<&Path>, _use_reoxide: bool) -> Result<()> {
    println!("Reading DOL file: {}", dol_file.display());

    let data = fs::read(dol_file)
        .with_context(|| format!("Failed to read DOL file: {}", dol_file.display()))?;
    let dol = DolFile::parse(&data, dol_file.to_str().unwrap_or("unknown.dol"))
        .context("Failed to parse DOL file")?;

    println!("DOL file parsed successfully");
    println!("  Text sections: {}", dol.text_sections.len());
    println!("  Data sections: {}", dol.data_sections.len());
    println!("  Entry point: 0x{:08X}", dol.entry_point);
    println!(
        "  BSS address: 0x{:08X}, size: 0x{:08X}",
        dol.bss_address, dol.bss_size
    );

    // Decode + discover + enrich (no Ghidra / external tool required).
    let (facts, report) = RecompilationPipeline::analyze_with_symbol_map(&dol, symbol_map)
        .context("Analysis failed")?;

    println!("\nAnalysis complete (naive discovery + enrichment):");
    println!("  Functions:            {}", report.functions);
    println!("  Leaf functions:       {}", report.leaf_functions);
    println!("  Functions with loops: {}", report.functions_with_loops);
    println!("  Instructions:         {}", report.total_instructions);
    println!(
        "  Instruction coverage: {:.1}% ({}/{} translated)",
        report.instruction_coverage() * 100.0,
        report.translated_instructions,
        report.total_instructions
    );

    // Show the 10 largest functions as a sample.
    let mut by_size: Vec<_> = facts.iter().collect();
    by_size.sort_by_key(|f| std::cmp::Reverse(f.instruction_count));
    println!("\n  Largest functions:");
    for f in by_size.iter().take(10) {
        println!(
            "    {} @ 0x{:08X}  {} instrs, {} calls{}{}",
            f.name,
            f.address,
            f.instruction_count,
            f.call_targets.len(),
            if f.is_leaf { ", leaf" } else { "" },
            if f.has_loop { ", loop" } else { "" },
        );
    }

    Ok(())
}

pub fn recompile_dol(
    dol_file: &Path,
    output_dir: Option<&Path>,
    symbol_map: Option<&Path>,
    _use_reoxide: bool,
) -> Result<()> {
    println!("Recompiling DOL file: {}", dol_file.display());

    let data = fs::read(dol_file)
        .with_context(|| format!("Failed to read DOL file: {}", dol_file.display()))?;
    let dol = DolFile::parse(&data, dol_file.to_str().unwrap_or("unknown.dol"))
        .context("Failed to parse DOL file")?;

    // Output: the `recompiled` library crate's lib.rs by default (so the whole
    // game becomes a compilable crate the `game` binary links). With --output-dir,
    // write <dir>/recompiled.rs instead.
    let output_file = match output_dir {
        Some(dir) => {
            fs::create_dir_all(dir).context("Failed to create output directory")?;
            dir.join("recompiled.rs")
        }
        None => PathBuf::from("recompiled/src/lib.rs"),
    };

    // Run the real decode -> analyze -> codegen pipeline (no Ghidra required).
    RecompilationPipeline::recompile_with_symbol_map(
        &dol,
        output_file.to_str().context("Invalid output path")?,
        symbol_map,
    )
    .context("Recompilation pipeline failed")?;

    println!("Generated Rust code written to: {}", output_file.display());

    Ok(())
}

pub fn build_dol(
    dol_file: &Path,
    output_dir: Option<&Path>,
    symbol_map: Option<&Path>,
    use_reoxide: bool,
) -> Result<()> {
    println!("Building recompiled game from: {}", dol_file.display());

    // Step 1: Recompile DOL -> Rust (decode + codegen, no Ghidra required).
    println!("Step 1/2: Recompiling to Rust...");
    recompile_dol(dol_file, output_dir, symbol_map, use_reoxide)?;

    // Step 2: Build the `game` crate into a native executable.
    println!("\nStep 2/2: Building the game crate...");
    let status = std::process::Command::new("cargo")
        .args(["build", "-p", "game"])
        .status()
        .context("Failed to run cargo build")?;

    if !status.success() {
        anyhow::bail!("Cargo build of `game` failed");
    }

    println!("Build complete! Executable: target/debug/game");

    Ok(())
}

pub fn prepare_disc(disc_image: &Path, output_dir: &Path, include_assets: bool) -> Result<()> {
    println!("Reading disc image: {}", disc_image.display());
    let mut disc = File::open(disc_image)
        .with_context(|| format!("Failed to open disc image: {}", disc_image.display()))?;
    let source_size = disc.metadata()?.len();
    if source_size < 0x440 {
        anyhow::bail!("Disc image is too small");
    }

    let mut header = [0u8; 0x440];
    disc.read_exact(&mut header)?;
    let game_id = String::from_utf8_lossy(&header[0..6]).into_owned();
    let dol_offset = read_u32_be(&header, 0x420) as u64;

    disc.seek(SeekFrom::Start(dol_offset))?;
    let mut dol_header = [0u8; 0x100];
    disc.read_exact(&mut dol_header)?;
    let dol_size = dol_file_size(&dol_header)?;
    if dol_offset.saturating_add(dol_size as u64) > source_size {
        anyhow::bail!("DOL extends past the end of the disc image");
    }

    disc.seek(SeekFrom::Start(dol_offset))?;
    let mut dol = vec![0u8; dol_size];
    disc.read_exact(&mut dol)?;
    fs::create_dir_all(output_dir).with_context(|| format!("creating {}", output_dir.display()))?;
    let dol_path = output_dir.join("main.dol");
    fs::write(&dol_path, &dol).with_context(|| format!("writing {}", dol_path.display()))?;

    disc.seek(SeekFrom::Start(0))?;
    let mut source_hasher = Sha256::new();
    let mut hash_buffer = vec![0u8; 4 * 1024 * 1024];
    loop {
        let read = disc.read(&mut hash_buffer)?;
        if read == 0 {
            break;
        }
        source_hasher.update(&hash_buffer[..read]);
    }
    let iso_sha256 = format!("{:x}", source_hasher.finalize());
    let dol_sha256 = format!("{:x}", Sha256::digest(&dol));
    let manifest = serde_json::json!({
        "game_id": game_id,
        "source_image": disc_image,
        "source_size": source_size,
        "source_sha256": iso_sha256,
        "dol_offset": dol_offset,
        "dol_size": dol_size,
        "dol_sha256": dol_sha256,
    });
    fs::write(
        output_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;

    println!(
        "Extracted {} ({} bytes, game ID {})",
        dol_path.display(),
        dol_size,
        manifest["game_id"].as_str().unwrap_or("unknown")
    );

    if include_assets {
        println!("Compressing disc filesystem for the local runtime...");
        let (file_count, archive_size) =
            build_disc_archive(&mut disc, &header, Path::new("game/assets.bin"))?;
        RecompilationPipeline::stage_embed_assets()?;
        println!(
            "Prepared {} disc files in game/assets.bin ({} bytes)",
            file_count, archive_size
        );
    }
    Ok(())
}

fn read_u32_be(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn dol_file_size(header: &[u8; 0x100]) -> Result<usize> {
    let mut end = 0usize;

    for index in 0..7 {
        let section_offset = read_u32_be(header, index * 4) as usize;
        let section_size = read_u32_be(header, 0x90 + index * 4) as usize;
        end = end.max(section_offset.saturating_add(section_size));
    }
    for index in 0..11 {
        let section_offset = read_u32_be(header, 0x1c + index * 4) as usize;
        let section_size = read_u32_be(header, 0xac + index * 4) as usize;
        end = end.max(section_offset.saturating_add(section_size));
    }
    if end < 0x100 {
        anyhow::bail!("DOL has no valid sections");
    }
    Ok(end)
}

struct DiscArchiveEntry {
    path: String,
    disc_offset: u64,
    size: u64,
}

struct ArchiveTocEntry {
    path: String,
    archive_offset: u64,
    compressed_size: u64,
    decompressed_size: u64,
}

fn build_disc_archive(
    disc: &mut File,
    header: &[u8; 0x440],
    output_path: &Path,
) -> Result<(usize, u64)> {
    let fst_offset = read_u32_be(header, 0x424) as u64;
    let fst_size = read_u32_be(header, 0x428) as usize;
    if fst_offset == 0 || fst_size < 12 {
        anyhow::bail!("Disc has an invalid filesystem table");
    }

    disc.seek(SeekFrom::Start(fst_offset))?;
    let mut fst = vec![0u8; fst_size];
    disc.read_exact(&mut fst)
        .context("reading disc filesystem table")?;
    let entry_count = read_u32_be(&fst, 8) as usize;
    let entries_size = entry_count
        .checked_mul(12)
        .context("filesystem table entry count overflow")?;
    if entry_count == 0 || entries_size > fst.len() {
        anyhow::bail!("Disc filesystem table is malformed");
    }

    let read_name = |name_offset: usize| -> Result<String> {
        let start = entries_size
            .checked_add(name_offset)
            .context("filesystem name offset overflow")?;
        if start >= fst.len() {
            anyhow::bail!("filesystem name points outside the string table");
        }
        let length = fst[start..]
            .iter()
            .position(|byte| *byte == 0)
            .context("unterminated filesystem name")?;
        Ok(String::from_utf8_lossy(&fst[start..start + length]).into_owned())
    };

    let mut files = Vec::new();
    let mut directories: Vec<(String, usize)> = Vec::new();
    for index in 1..entry_count {
        while directories
            .last()
            .is_some_and(|(_, end_index)| index >= *end_index)
        {
            directories.pop();
        }

        let entry = index * 12;
        let name_offset = ((fst[entry + 1] as usize) << 16)
            | ((fst[entry + 2] as usize) << 8)
            | fst[entry + 3] as usize;
        let name = read_name(name_offset)?;
        if fst[entry] == 1 {
            directories.push((name, read_u32_be(&fst, entry + 8) as usize));
            continue;
        }

        let mut path = String::new();
        for (directory, _) in &directories {
            path.push_str(directory);
            path.push('/');
        }
        path.push_str(&name);
        files.push(DiscArchiveEntry {
            path,
            disc_offset: read_u32_be(&fst, entry + 4) as u64,
            size: read_u32_be(&fst, entry + 8) as u64,
        });
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut archive = BufWriter::new(
        File::create(output_path)
            .with_context(|| format!("creating archive {}", output_path.display()))?,
    );
    archive.write_all(b"GCFS")?;
    archive.write_all(&1u32.to_le_bytes())?;
    archive.write_all(&(files.len() as u32).to_le_bytes())?;
    archive.write_all(&0u64.to_le_bytes())?;

    let mut toc = Vec::with_capacity(files.len());
    for file in &files {
        let archive_offset = archive.stream_position()?;
        disc.seek(SeekFrom::Start(file.disc_offset))?;
        let copied = {
            let mut source = (&mut *disc).take(file.size);
            let mut encoder = zstd::stream::write::Encoder::new(&mut archive, 3)
                .with_context(|| format!("starting compression for {}", file.path))?;
            let copied = io::copy(&mut source, &mut encoder)
                .with_context(|| format!("compressing {}", file.path))?;
            encoder
                .finish()
                .with_context(|| format!("finishing compression for {}", file.path))?;
            copied
        };
        if copied != file.size {
            anyhow::bail!(
                "disc file {} ended early (expected {}, read {})",
                file.path,
                file.size,
                copied
            );
        }
        let compressed_size = archive.stream_position()? - archive_offset;
        toc.push(ArchiveTocEntry {
            path: file.path.clone(),
            archive_offset,
            compressed_size,
            decompressed_size: file.size,
        });
    }

    let toc_offset = archive.stream_position()?;
    for entry in &toc {
        let path = entry.path.as_bytes();
        let path_len = u16::try_from(path.len()).context("disc path exceeds 65535 bytes")?;
        archive.write_all(&path_len.to_le_bytes())?;
        archive.write_all(path)?;
        archive.write_all(&entry.archive_offset.to_le_bytes())?;
        archive.write_all(&entry.compressed_size.to_le_bytes())?;
        archive.write_all(&entry.decompressed_size.to_le_bytes())?;
    }
    let archive_size = archive.stream_position()?;
    archive.seek(SeekFrom::Start(12))?;
    archive.write_all(&toc_offset.to_le_bytes())?;
    archive.flush()?;

    Ok((files.len(), archive_size))
}

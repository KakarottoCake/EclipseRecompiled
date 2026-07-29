use std::path::PathBuf;

/// Find the private GCFS archive without embedding it in the executable.
pub fn archive_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("GCRECOMP_ASSETS").map(PathBuf::from) {
        return path.is_file().then_some(path);
    }
    if let Ok(executable) = std::env::current_exe() {
        let sibling = executable.with_file_name("assets.bin");
        if sibling.is_file() {
            return Some(sibling);
        }
    }
    let workspace = PathBuf::from("game").join("assets.bin");
    workspace.is_file().then_some(workspace)
}

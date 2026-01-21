use std::fs;
use std::path::{Path, PathBuf};
use std::io;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "wav", "flac", "ogg", "m4a", "aac"];

/// Get the available files in a directory, sorted by type (dir first) then name.
pub fn get_directory_content(path: &Path) -> io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    if let Ok(read_dir) = fs::read_dir(path) {
        for entry_result in read_dir {
            if let Ok(entry) = entry_result {
                let path = entry.path();
                let is_dir = path.is_dir();
                
                // Filter: Include directories and supported audio files
                let should_include = is_dir || path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false);

                if should_include {
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("???")
                        .to_string();

                    entries.push(FileEntry {
                        name,
                        path,
                        is_dir,
                    });
                }
            }
        }
    }

    // Sort: Directories first, then alphabetical
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    // Add ".." entry for parent directory if not at root
    if let Some(parent) = path.parent() {
        entries.insert(0, FileEntry {
            name: "..".to_string(),
            path: parent.to_path_buf(),
            is_dir: true,
        });
    }

    Ok(entries)
}
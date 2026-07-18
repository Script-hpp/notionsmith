use std::path::{ Path, PathBuf };

/// A PDF found in the watch folder, already parsed into its subject prefix.
pub struct LocalFile {
    pub path: PathBuf,
    pub filename: String,
    pub prefix: String,
}

/// Extracts the subject prefix from a Notein export filename, e.g.
/// `MATHE1_Test1.pdf` -> `Some("MATHE1")`. Filenames without a `_` (or without a
/// `.pdf` extension) don't belong to any subject and are skipped.
fn parse_prefix(filename: &str) -> Option<String> {
    let stem = filename.strip_suffix(".pdf").or_else(|| filename.strip_suffix(".PDF"))?;
    let (prefix, _) = stem.split_once('_')?;
    if prefix.is_empty() {
        return None;
    }
    Some(prefix.to_uppercase())
}

/// Scans the watch folder (non-recursive) for PDFs that follow the
/// `<PREFIX>_<anything>.pdf` naming convention. Files that don't match are silently
/// skipped; they aren't Notein exports we know how to route.
pub fn scan_watch_dir(dir: &Path) -> Result<Vec<LocalFile>, std::io::Error> {
    let mut files = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let filename = entry.file_name().to_string_lossy().to_string();
        let Some(prefix) = parse_prefix(&filename) else { continue };

        files.push(LocalFile { path: entry.path(), filename, prefix });
    }

    Ok(files)
}

#[cfg(test)]
#[path = "notein/tests.rs"]
mod tests;

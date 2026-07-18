use std::path::{ Path, PathBuf };

/// A PDF found in the watch folder, already parsed into its subject prefix.
pub struct LocalFile {
    pub path: PathBuf,
    /// The original filename, e.g. `MATHE1_TEST.pdf`. Kept as-is for the file
    /// attachment (Notion shows this as the attached file's name).
    pub filename: String,
    /// The filename with the `<PREFIX>_` part and the extension stripped, e.g.
    /// `TEST`. Used as the Notion page title, since the course already has its own
    /// property and doesn't need to be repeated in the title.
    pub title: String,
    pub prefix: String,
}

/// Splits a Notein export filename into its subject prefix and display title, e.g.
/// `MATHE1_TEST.pdf` -> `Some(("MATHE1", "TEST"))`. Filenames without a `_` (or
/// without a `.pdf` extension) don't belong to any subject and are skipped.
fn parse_prefix_and_title(filename: &str) -> Option<(String, String)> {
    let stem = filename.strip_suffix(".pdf").or_else(|| filename.strip_suffix(".PDF"))?;
    let (prefix, title) = stem.split_once('_')?;
    if prefix.is_empty() {
        return None;
    }
    Some((prefix.to_uppercase(), title.to_string()))
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
        let Some((prefix, title)) = parse_prefix_and_title(&filename) else { continue };

        files.push(LocalFile { path: entry.path(), filename, title, prefix });
    }

    Ok(files)
}

#[cfg(test)]
#[path = "notein/tests.rs"]
mod tests;

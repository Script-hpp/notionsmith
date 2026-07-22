use crate::notein::LocalFile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HistoryRecord {
    pub title: String,
    pub filename: String,
    pub prefix: String,
    pub file_size: u64,
    pub modified_secs: u64,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SyncHistory {
    records: HashMap<String, HistoryRecord>,
}

impl SyncHistory {
    pub fn load(path: &Path) -> Self {
        if let Ok(content) = fs::read_to_string(path)
            && let Ok(history) = serde_json::from_str(&content)
        {
            return history;
        }
        Self::default()
    }

    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Checks whether the exact file version (`title`, `size`, `mtime`) has already been uploaded.
    pub fn is_already_synced(&self, file: &LocalFile, file_size: u64, modified_secs: u64) -> bool {
        if let Some(record) = self.records.get(&file.title) {
            return record.file_size == file_size && record.modified_secs == modified_secs;
        }
        false
    }

    /// Records or updates a successfully synced file in history.
    pub fn record(&mut self, file: &LocalFile, file_size: u64, modified_secs: u64) {
        self.records.insert(
            file.title.clone(),
            HistoryRecord {
                title: file.title.clone(),
                filename: file.filename.clone(),
                prefix: file.prefix.clone(),
                file_size,
                modified_secs,
            },
        );
    }
}

#[cfg(test)]
#[path = "history/tests.rs"]
mod tests;

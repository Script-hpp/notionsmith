use super::*;
use std::path::PathBuf;

fn sample_local_file() -> LocalFile {
    LocalFile {
        path: PathBuf::from("MATHE1_Test1.pdf"),
        filename: "MATHE1_Test1.pdf".to_string(),
        title: "Test1".to_string(),
        prefix: "MATHE1".to_string(),
    }
}

#[test]
fn loads_default_when_file_missing() {
    let history = SyncHistory::load(Path::new("nonexistent_path_history.json"));
    assert!(history.records.is_empty());
}

#[test]
fn records_and_checks_synced_status() {
    let mut history = SyncHistory::default();
    let file = sample_local_file();

    assert!(!history.is_already_synced(&file, 1024, 1690000000));

    history.record(&file, 1024, 1690000000);
    assert!(history.is_already_synced(&file, 1024, 1690000000));
}

#[test]
fn detects_changed_file_size_or_mtime() {
    let mut history = SyncHistory::default();
    let file = sample_local_file();

    history.record(&file, 1024, 1690000000);

    // Different size -> not synced
    assert!(!history.is_already_synced(&file, 2048, 1690000000));
    // Different mtime -> not synced
    assert!(!history.is_already_synced(&file, 1024, 1690000001));
}

#[test]
fn roundtrips_history_to_file() {
    let temp_dir = std::env::temp_dir().join("notionsmith_history_test");
    let _ = fs::remove_dir_all(&temp_dir);
    let history_path = temp_dir.join("history.json");

    let mut history = SyncHistory::default();
    let file = sample_local_file();
    history.record(&file, 1024, 1690000000);

    history.save(&history_path).expect("save should succeed");

    let loaded = SyncHistory::load(&history_path);
    assert!(loaded.is_already_synced(&file, 1024, 1690000000));

    let _ = fs::remove_dir_all(&temp_dir);
}

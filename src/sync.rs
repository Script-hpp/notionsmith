use crate::notein;
use crate::notion::{ self, NewNote, PropertyNames };
use std::collections::HashMap;
use std::path::PathBuf;

/// Everything the sync cycle needs, loaded once at startup from the environment.
pub struct SyncConfig {
    pub watch_dir: PathBuf,
    pub history_path: PathBuf,
    pub notion_token: String,
    pub database_id: String,
    pub title_property: String,
    pub file_property: String,
    pub course_property: String,
    pub status_property: Option<String>,
    pub status_value: Option<String>,
    /// Subject prefix (e.g. "MATHE1") -> exact course select option (e.g. "Mathematik I").
    pub course_map: HashMap<String, String>,
}

/// Runs exactly one poll cycle: scans the watch folder, and uploads every PDF whose
/// prefix maps to a known course and whose display title isn't already a page title
/// in the database.
///
/// Returns an error instead of terminating the process; the main loop catches it and
/// continues at the next interval.
pub async fn run_sync_cycle(
    client: &reqwest::Client,
    config: &SyncConfig
) -> Result<(), Box<dyn std::error::Error>> {
    let local_files = notein::scan_watch_dir(&config.watch_dir)?;

    let existing_titles = notion
        ::fetch_existing_titles(client, &config.notion_token, &config.database_id, &config.title_property).await?;

    let mut history = crate::history::SyncHistory::load(&config.history_path);
    let mut history_changed = false;

    let properties = PropertyNames {
        title: &config.title_property,
        file: &config.file_property,
        course: &config.course_property,
        status: config.status_property.as_deref(),
    };

    for file in &local_files {
        let Some(course) = config.course_map.get(&file.prefix) else {
            println!("  - Skipping '{}': no course configured for prefix '{}'.", file.filename, file.prefix);
            continue;
        };

        let metadata = match std::fs::metadata(&file.path) {
            Ok(m) => m,
            Err(e) => {
                println!("  ✗ Failed to read metadata for '{}': {}", file.filename, e);
                continue;
            }
        };
        let file_size = metadata.len();
        let modified_secs = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if existing_titles.contains(&file.title) {
            println!("  ✓ '{}' already uploaded.", file.title);
            if !history.is_already_synced(file, file_size, modified_secs) {
                history.record(file, file_size, modified_secs);
                history_changed = true;
            }
            continue;
        }

        if history.is_already_synced(file, file_size, modified_secs) {
            println!("  ✓ '{}' was previously uploaded (skipping re-upload of deleted/renamed note).", file.title);
            continue;
        }

        println!("➔ Uploading '{}' ({})...", file.title, course);
        let bytes = std::fs::read(&file.path)?;
        match notion::upload_file(client, &config.notion_token, &file.filename, bytes).await {
            Ok(file_upload_id) => {
                let note = NewNote {
                    title: &file.title,
                    filename: &file.filename,
                    file_upload_id: &file_upload_id,
                    course,
                    status: config.status_value.as_deref(),
                };
                match
                    notion::create_page(
                        client,
                        &config.notion_token,
                        &config.database_id,
                        &properties,
                        &note
                    ).await
                {
                    Ok(()) => {
                        println!("  ✓ Created page for '{}'.", file.title);
                        history.record(file, file_size, modified_secs);
                        history_changed = true;
                    }
                    Err(e) => println!("  ✗ Failed to create page for '{}': {}", file.title, e),
                }
            }
            Err(e) => println!("  ✗ Failed to upload '{}': {}", file.filename, e),
        }
    }

    if history_changed
        && let Err(e) = history.save(&config.history_path)
    {
        println!("  ⚠ Failed to save history to {:?}: {}", config.history_path, e);
    }

    Ok(())
}

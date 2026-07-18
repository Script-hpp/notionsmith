use crate::notein;
use crate::notion;
use std::collections::HashMap;
use std::path::PathBuf;

/// Everything the sync cycle needs, loaded once at startup from the environment.
pub struct SyncConfig {
    pub watch_dir: PathBuf,
    pub notion_token: String,
    pub title_property: String,
    pub file_property: String,
    /// Subject prefix (e.g. "MATHE1") -> Notion database id.
    pub database_map: HashMap<String, String>,
}

/// Runs exactly one poll cycle: scans the watch folder, and uploads every PDF whose
/// prefix maps to a known database and whose filename isn't already a page title
/// there.
///
/// Returns an error instead of terminating the process; the main loop catches it and
/// continues at the next interval.
pub async fn run_sync_cycle(
    client: &reqwest::Client,
    config: &SyncConfig
) -> Result<(), Box<dyn std::error::Error>> {
    let local_files = notein::scan_watch_dir(&config.watch_dir)?;

    // Cache existing titles per database: multiple local files can share a prefix,
    // and re-fetching for every single file would multiply Notion API calls.
    let mut existing_titles_by_prefix: HashMap<&str, std::collections::HashSet<String>> = HashMap::new();

    for file in &local_files {
        let Some(database_id) = config.database_map.get(&file.prefix) else {
            println!("  - Skipping '{}': no database configured for prefix '{}'.", file.filename, file.prefix);
            continue;
        };

        if !existing_titles_by_prefix.contains_key(file.prefix.as_str()) {
            let titles = notion
                ::fetch_existing_titles(client, &config.notion_token, database_id, &config.title_property).await?;
            existing_titles_by_prefix.insert(&file.prefix, titles);
        }

        let existing_titles = &existing_titles_by_prefix[file.prefix.as_str()];
        if existing_titles.contains(&file.filename) {
            println!("  ✓ '{}' already uploaded.", file.filename);
            continue;
        }

        println!("➔ Uploading '{}' to database for prefix '{}'...", file.filename, file.prefix);
        let bytes = std::fs::read(&file.path)?;
        match notion::upload_file(client, &config.notion_token, &file.filename, bytes).await {
            Ok(file_upload_id) => {
                match
                    notion::create_page(
                        client,
                        &config.notion_token,
                        database_id,
                        &config.title_property,
                        &config.file_property,
                        &file.filename,
                        &file_upload_id
                    ).await
                {
                    Ok(()) => println!("  ✓ Created page for '{}'.", file.filename),
                    Err(e) => println!("  ✗ Failed to create page for '{}': {}", file.filename, e),
                }
            }
            Err(e) => println!("  ✗ Failed to upload '{}': {}", file.filename, e),
        }
    }

    Ok(())
}

mod configure;
mod notein;
mod notion;
mod sync;

use std::collections::HashMap;
use std::time::Duration;
use sync::SyncConfig;
use tokio::time::sleep;

/// Resolves `$XDG_CONFIG_HOME`, falling back to `~/.config` per XDG convention.
fn xdg_config_dir() -> Option<std::path::PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return Some(std::path::PathBuf::from(xdg));
    }
    std::env::var("HOME").ok().map(|home| std::path::PathBuf::from(home).join(".config"))
}

/// Loads config from `~/.config/notionsmith/.env` if present, otherwise from `.env`
/// in the current directory.
///
/// Deliberately not using `dotenvy::dotenv()` here: it walks up parent directories
/// looking for a `.env`, which is a footgun for an installed binary that could
/// silently pick up an unrelated `.env` from some ancestor directory instead of
/// failing cleanly.
fn load_config() {
    if let Some(path) = xdg_config_dir().map(|dir| dir.join("notionsmith").join(".env"))
        && dotenvy::from_path(&path).is_ok()
    {
        return;
    }
    // Not using `let _ =` here: a malformed .env (e.g. an unquoted value containing
    // a space) fails silently otherwise, and every required env var then panics with
    // a message that doesn't point at the real cause.
    if let Err(e) = dotenvy::from_filename(".env") {
        println!("  ⚠ Could not load .env: {}", e);
    }
}

/// Builds the prefix -> course select value map from every `NOTEIN_COURSE_<PREFIX>`
/// environment variable, e.g. `NOTEIN_COURSE_MATHE1=Mathematik I` maps the `MATHE1_`
/// filename prefix to that exact select option. New subjects only need a new env
/// var, no code change.
fn load_course_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (key, value) in std::env::vars() {
        if let Some(prefix) = key.strip_prefix("NOTEIN_COURSE_") {
            map.insert(prefix.to_string(), value);
        }
    }
    map
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_config();

    let client = reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?;

    let notion_token = std::env::var("NOTION_TOKEN").expect("NOTION_TOKEN must be set");
    let database_id = std::env::var("NOTION_DATABASE_ID").expect("NOTION_DATABASE_ID must be set");
    let course_property = std::env
        ::var("NOTION_COURSE_PROPERTY")
        .unwrap_or_else(|_| "Course".to_string());
    let title_property = std::env
        ::var("NOTION_TITLE_PROPERTY")
        .unwrap_or_else(|_| "Name".to_string());
    let watch_dir = std::env::var("NOTEIN_WATCH_DIR").expect("NOTEIN_WATCH_DIR must be set");

    if std::env::args().nth(1).as_deref() == Some("configure") {
        return configure::run(
            &client,
            &notion_token,
            &database_id,
            &course_property,
            &title_property,
            std::path::Path::new(&watch_dir)
        ).await;
    }

    println!("notionsmith sync daemon is running!");

    let file_property = std::env
        ::var("NOTION_FILE_PROPERTY")
        .unwrap_or_else(|_| "Files & media".to_string());
    // Both optional, and only used together: without a configured property name
    // there's nowhere to write the status value.
    let status_property = std::env::var("NOTION_STATUS_PROPERTY").ok();
    let status_value = std::env::var("NOTION_STATUS_VALUE").ok();

    let course_map = load_course_map();
    if course_map.is_empty() {
        panic!("No NOTEIN_COURSE_<PREFIX> environment variables found; nothing to sync to.");
    }
    println!("Configured subjects: {}", course_map.keys().cloned().collect::<Vec<_>>().join(", "));

    let poll_interval = Duration::from_secs(
        std::env
            ::var("SYNC_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(60)
    );

    let config = SyncConfig {
        watch_dir: std::path::PathBuf::from(watch_dir),
        notion_token,
        database_id,
        title_property,
        file_property,
        course_property,
        status_property,
        status_value,
        course_map,
    };

    loop {
        println!("\n--- Scanning watch folder ---");

        // An error (e.g. network timeout, API 5xx) no longer terminates the process,
        // only this cycle. The next interval will retry.
        if let Err(e) = sync::run_sync_cycle(&client, &config).await {
            println!("  ✗ Sync cycle failed: {}", e);
        }

        println!("Waiting {} seconds until next scan...", poll_interval.as_secs());
        sleep(poll_interval).await;
    }
}

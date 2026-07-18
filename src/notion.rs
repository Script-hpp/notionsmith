pub mod model;

use model::{ CreatePageResponse, DatabaseQueryResponse, FileUploadCreateResponse };
use reqwest::multipart;
use std::collections::HashSet;

const NOTION_VERSION: &str = "2022-06-28";

/// Title of the maintenance page `configure` keeps up to date inside the notes
/// database with a plain-text prefix -> course reference. Kept in Notion itself
/// (not only as a local file) since Notion is the one thing every user of this tool
/// has, regardless of whether they also use Syncthing or anything else to sync the
/// watch folder.
pub const REFERENCE_PAGE_TITLE: &str = "📋 Notionsmith Präfixe";

/// Notion property names, resolved once from config and reused for every page
/// created in this database.
pub struct PropertyNames<'a> {
    pub title: &'a str,
    pub file: &'a str,
    pub course: &'a str,
    /// `None` if no status property is configured; the status property is optional.
    pub status: Option<&'a str>,
}

/// One local file, ready to become a Notion page.
pub struct NewNote<'a> {
    pub title: &'a str,
    pub filename: &'a str,
    pub file_upload_id: &'a str,
    /// The `Kurs` select value. Must match an existing option exactly, otherwise
    /// Notion silently creates a new (randomly colored) option instead of reusing
    /// one.
    pub course: &'a str,
    pub status: Option<&'a str>,
}

/// Fetches the exact names of every existing option on the `Kurs` select property,
/// so `configure` can offer them for picking instead of the user having to retype
/// them (and risking a typo that makes Notion silently create a new option).
pub async fn fetch_course_options(
    client: &reqwest::Client,
    token: &str,
    database_id: &str,
    course_property: &str
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let url = format!("https://api.notion.com/v1/databases/{}", database_id);
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Notion-Version", NOTION_VERSION)
        .send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Failed to fetch database schema (status {}): {}", status, text).into());
    }

    let schema = response.json::<serde_json::Value>().await?;
    let options = schema
        .get("properties")
        .and_then(|properties| properties.get(course_property))
        .and_then(|property| property.get("select"))
        .and_then(|select| select.get("options"))
        .and_then(|options| options.as_array())
        .ok_or_else(|| format!("'{}' is not a select property on this database", course_property))?;

    Ok(
        options
            .iter()
            .filter_map(|option| option.get("name")?.as_str().map(str::to_string))
            .collect()
    )
}

fn extract_title(properties: &serde_json::Value, title_property: &str) -> Option<String> {
    properties
        .get(title_property)?
        .get("title")?
        .as_array()?
        .first()?
        .get("plain_text")?
        .as_str()
        .map(str::to_string)
}

/// Fetches the title of every page already in the target database, so newly scanned
/// local files can be matched against what's already been uploaded.
pub async fn fetch_existing_titles(
    client: &reqwest::Client,
    token: &str,
    database_id: &str,
    title_property: &str
) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let mut titles = HashSet::new();
    let mut start_cursor: Option<String> = None;
    let url = format!("https://api.notion.com/v1/databases/{}/query", database_id);

    loop {
        let mut body = serde_json::json!({ "page_size": 100 });
        if let Some(cursor) = &start_cursor {
            body["start_cursor"] = serde_json::json!(cursor);
        }

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Notion-Version", NOTION_VERSION)
            .json(&body)
            .send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Failed to query database (status {}): {}", status, text).into());
        }

        let response = response.json::<DatabaseQueryResponse>().await?;

        for page in response.results {
            if let Some(title) = extract_title(&page.properties, title_property) {
                titles.insert(title);
            }
        }

        if !response.has_more {
            break;
        }
        match response.next_cursor {
            Some(cursor) => start_cursor = Some(cursor),
            None => break,
        }
    }

    Ok(titles)
}

/// Uploads a file's bytes to Notion via the two-step File Upload API (create, then
/// send) and returns the file upload id, ready to be attached to a page property.
///
/// Only single-part upload is implemented; Notion caps this at 20 MB per file. A PDF
/// bigger than that would need the multi-part upload flow instead, which this does
/// not (yet) support.
pub async fn upload_file(
    client: &reqwest::Client,
    token: &str,
    filename: &str,
    bytes: Vec<u8>
) -> Result<String, Box<dyn std::error::Error>> {
    let create_response = client
        .post("https://api.notion.com/v1/file_uploads")
        .header("Authorization", format!("Bearer {}", token))
        .header("Notion-Version", NOTION_VERSION)
        .json(&serde_json::json!({ "filename": filename, "content_type": "application/pdf" }))
        .send().await?;

    if !create_response.status().is_success() {
        let status = create_response.status();
        let text = create_response.text().await.unwrap_or_default();
        return Err(format!("Failed to create file upload (status {}): {}", status, text).into());
    }

    let created = create_response.json::<FileUploadCreateResponse>().await?;

    let file_part = multipart::Part::bytes(bytes).file_name(filename.to_string()).mime_str("application/pdf")?;
    let form = multipart::Form::new().part("file", file_part);

    let send_response = client
        .post(&created.upload_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Notion-Version", NOTION_VERSION)
        .multipart(form)
        .send().await?;

    if !send_response.status().is_success() {
        let status = send_response.status();
        let text = send_response.text().await.unwrap_or_default();
        return Err(format!("Failed to send file bytes (status {}): {}", status, text).into());
    }

    Ok(created.id)
}

/// Finds the id of the page with an exact title match in the target database, using
/// a server-side filter rather than paginating through everything client-side.
async fn find_page_id_by_title(
    client: &reqwest::Client,
    token: &str,
    database_id: &str,
    title_property: &str,
    title: &str
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let url = format!("https://api.notion.com/v1/databases/{}/query", database_id);
    let body = serde_json::json!({
        "filter": { "property": title_property, "title": { "equals": title } },
        "page_size": 1
    });

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Notion-Version", NOTION_VERSION)
        .json(&body)
        .send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Failed to search for '{}' (status {}): {}", title, status, text).into());
    }

    let response = response.json::<DatabaseQueryResponse>().await?;
    Ok(response.results.into_iter().next().map(|page| page.id))
}

fn paragraph_block(text: &str) -> serde_json::Value {
    serde_json::json!({
        "object": "block",
        "type": "paragraph",
        "paragraph": { "rich_text": [{ "type": "text", "text": { "content": text } }] }
    })
}

/// Replaces a page's block content wholesale: every existing child block is
/// archived, then the given lines are appended as fresh paragraph blocks. Used to
/// keep the reference page's content in sync on every `configure` save, rather than
/// accumulating a new copy of the list underneath the old one each time.
async fn replace_page_children(
    client: &reqwest::Client,
    token: &str,
    page_id: &str,
    lines: &[String]
) -> Result<(), Box<dyn std::error::Error>> {
    let list_url = format!("https://api.notion.com/v1/blocks/{}/children?page_size=100", page_id);
    let append_url = format!("https://api.notion.com/v1/blocks/{}/children", page_id);
    let response = client
        .get(&list_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Notion-Version", NOTION_VERSION)
        .send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Failed to list reference page blocks (status {}): {}", status, text).into());
    }

    let existing = response.json::<serde_json::Value>().await?;
    if let Some(blocks) = existing.get("results").and_then(|r| r.as_array()) {
        for block in blocks {
            let Some(block_id) = block.get("id").and_then(|id| id.as_str()) else { continue };
            let delete_url = format!("https://api.notion.com/v1/blocks/{}", block_id);
            client
                .delete(&delete_url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Notion-Version", NOTION_VERSION)
                .send().await?;
        }
    }

    let new_children: Vec<serde_json::Value> = lines.iter().map(|line| paragraph_block(line)).collect();
    let append_response = client
        .patch(&append_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Notion-Version", NOTION_VERSION)
        .json(&serde_json::json!({ "children": new_children }))
        .send().await?;

    if !append_response.status().is_success() {
        let status = append_response.status();
        let text = append_response.text().await.unwrap_or_default();
        return Err(format!("Failed to write reference page content (status {}): {}", status, text).into());
    }

    Ok(())
}

/// Creates or updates the reference page (see `REFERENCE_PAGE_TITLE`) so it always
/// reflects the current prefix -> course mapping, directly inside the notes
/// database. Its title never matches a real filename-derived title, so it's never
/// picked up by `fetch_existing_titles`'s dedup check.
pub async fn upsert_reference_page(
    client: &reqwest::Client,
    token: &str,
    database_id: &str,
    title_property: &str,
    lines: &[String]
) -> Result<(), Box<dyn std::error::Error>> {
    let existing_id = find_page_id_by_title(
        client,
        token,
        database_id,
        title_property,
        REFERENCE_PAGE_TITLE
    ).await?;

    let page_id = match existing_id {
        Some(id) => id,
        None => {
            let body = serde_json::json!({
                "parent": { "database_id": database_id },
                "properties": {
                    title_property: {
                        "title": [{ "text": { "content": REFERENCE_PAGE_TITLE } }]
                    }
                }
            });

            let response = client
                .post("https://api.notion.com/v1/pages")
                .header("Authorization", format!("Bearer {}", token))
                .header("Notion-Version", NOTION_VERSION)
                .json(&body)
                .send().await?;

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                return Err(format!("Failed to create reference page (status {}): {}", status, text).into());
            }

            response.json::<CreatePageResponse>().await?.id
        }
    };

    replace_page_children(client, token, &page_id, lines).await
}

/// Creates a new page in the target database: titled, tagged with its course (and
/// optionally a status), with the uploaded PDF attached under the configured files
/// property.
pub async fn create_page(
    client: &reqwest::Client,
    token: &str,
    database_id: &str,
    properties: &PropertyNames<'_>,
    note: &NewNote<'_>
) -> Result<(), Box<dyn std::error::Error>> {
    let mut page_properties = serde_json::json!({
        properties.title: {
            "title": [{ "text": { "content": note.title } }]
        },
        properties.file: {
            "files": [{
                "name": note.filename,
                "type": "file_upload",
                "file_upload": { "id": note.file_upload_id }
            }]
        },
        properties.course: {
            "select": { "name": note.course }
        }
    });

    if let (Some(status_property), Some(status_value)) = (properties.status, note.status) {
        page_properties[status_property] = serde_json::json!({ "select": { "name": status_value } });
    }

    let body = serde_json::json!({
        "parent": { "database_id": database_id },
        "properties": page_properties
    });

    let response = client
        .post("https://api.notion.com/v1/pages")
        .header("Authorization", format!("Bearer {}", token))
        .header("Notion-Version", NOTION_VERSION)
        .json(&body)
        .send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Failed to create page (status {}): {}", status, text).into());
    }

    Ok(())
}

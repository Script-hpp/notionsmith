pub mod model;

use model::{ DatabaseQueryResponse, FileUploadCreateResponse };
use reqwest::multipart;
use std::collections::HashSet;

const NOTION_VERSION: &str = "2022-06-28";

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
            .send().await?
            .json::<DatabaseQueryResponse>().await?;

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

/// Creates a new page in the target database, titled after the local filename, with
/// the uploaded PDF attached under the configured files property.
pub async fn create_page(
    client: &reqwest::Client,
    token: &str,
    database_id: &str,
    title_property: &str,
    file_property: &str,
    filename: &str,
    file_upload_id: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let body = serde_json::json!({
        "parent": { "database_id": database_id },
        "properties": {
            title_property: {
                "title": [{ "text": { "content": filename } }]
            },
            file_property: {
                "files": [{
                    "name": filename,
                    "type": "file_upload",
                    "file_upload": { "id": file_upload_id }
                }]
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
        return Err(format!("Failed to create page (status {}): {}", status, text).into());
    }

    Ok(())
}

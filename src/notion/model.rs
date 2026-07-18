use serde::Deserialize;

/// One page of results from a Notion database query. Properties are kept as raw JSON
/// since the title/file property names are user-configured, not fixed field names.
#[derive(Deserialize)]
pub struct DatabaseQueryResponse {
    pub results: Vec<PageResult>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Deserialize)]
pub struct PageResult {
    pub properties: serde_json::Value,
}

/// Response from `POST /v1/file_uploads`: the pending upload's id and the URL its
/// bytes must be sent to next.
#[derive(Deserialize)]
pub struct FileUploadCreateResponse {
    pub id: String,
    pub upload_url: String,
}

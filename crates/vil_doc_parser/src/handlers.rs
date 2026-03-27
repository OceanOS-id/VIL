use vil_server::prelude::*;

use crate::{auto_parse, ParsedDoc};

#[derive(Debug, Deserialize)]
pub struct ParseRequest {
    pub content: String,
    pub file_type: String,
}

#[derive(Debug, Serialize)]
pub struct ParseResponseBody {
    pub document: ParsedDoc,
    pub section_count: usize,
}

#[derive(Debug, Serialize)]
pub struct FormatsResponseBody {
    pub supported_formats: Vec<String>,
    pub version: String,
}

pub async fn parse_handler(
    body: ShmSlice,
) -> HandlerResult<VilResponse<ParseResponseBody>> {
    let req: ParseRequest = body.json().expect("invalid JSON");
    if req.content.is_empty() {
        return Err(VilError::bad_request("content must not be empty"));
    }
    let doc = auto_parse(req.content.as_bytes(), &req.file_type)
        .map_err(|e| VilError::bad_request(e.to_string()))?;
    let section_count = doc.sections.len();
    Ok(VilResponse::ok(ParseResponseBody { document: doc, section_count }))
}

pub async fn formats_handler() -> HandlerResult<VilResponse<FormatsResponseBody>> {
    Ok(VilResponse::ok(FormatsResponseBody {
        supported_formats: vec![
            "md".into(), "markdown".into(),
            "html".into(), "htm".into(),
            "csv".into(), "tsv".into(),
            "txt".into(), "text".into(),
        ],
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}

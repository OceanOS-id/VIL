use vil_server::prelude::*;

use crate::{
    JsonOutputParser, MarkdownOutputParser, OutputParser, ParsedOutput, RegexOutputParser,
};

#[derive(Debug, Deserialize)]
pub struct ParseRequest {
    pub text: String,
    #[serde(default = "default_format")]
    pub format: String,
    pub pattern: Option<String>,
}

fn default_format() -> String {
    "json".into()
}

#[derive(Debug, Serialize)]
pub struct ParseResponseBody {
    pub format: String,
    pub result: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ParserStatsBody {
    pub supported_formats: Vec<String>,
    pub version: String,
}

pub async fn parse_handler(body: ShmSlice) -> HandlerResult<VilResponse<ParseResponseBody>> {
    let req: ParseRequest = body.json().expect("invalid JSON");
    if req.text.is_empty() {
        return Err(VilError::bad_request("text must not be empty"));
    }

    let parsed: ParsedOutput = match req.format.as_str() {
        "json" => {
            let parser = JsonOutputParser;
            parser
                .parse(&req.text)
                .map_err(|e| VilError::bad_request(format!("json parse failed: {e}")))?
        }
        "markdown" => {
            let parser = MarkdownOutputParser;
            parser
                .parse(&req.text)
                .map_err(|e| VilError::bad_request(format!("markdown parse failed: {e}")))?
        }
        "regex" => {
            let pattern = req
                .pattern
                .as_deref()
                .ok_or_else(|| VilError::bad_request("regex format requires a 'pattern' field"))?;
            let parser = RegexOutputParser::new(pattern)
                .map_err(|e| VilError::bad_request(format!("invalid regex: {e}")))?;
            parser
                .parse(&req.text)
                .map_err(|e| VilError::bad_request(format!("regex parse failed: {e}")))?
        }
        other => {
            return Err(VilError::bad_request(format!(
                "unsupported format: {other}"
            )));
        }
    };

    let value = match &parsed {
        ParsedOutput::Json(v) => v.clone(),
        ParsedOutput::Text(t) => serde_json::Value::String(t.clone()),
        ParsedOutput::Structured(map) => {
            serde_json::to_value(map).unwrap_or(serde_json::Value::Null)
        }
    };

    Ok(VilResponse::ok(ParseResponseBody {
        format: req.format,
        result: value,
    }))
}

pub async fn stats_handler() -> HandlerResult<VilResponse<ParserStatsBody>> {
    Ok(VilResponse::ok(ParserStatsBody {
        supported_formats: vec!["json".into(), "regex".into(), "markdown".into()],
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}

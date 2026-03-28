// =============================================================================
// VIL Server Web — OpenAPI 3.0 Auto-Generation
// =============================================================================
//
// Generates OpenAPI 3.0 specification from registered routes.
// Routes are collected at server startup and exposed at /openapi.json.
//
// This is a programmatic approach — routes are registered manually
// with metadata. A future proc-macro (#[vil_handler]) could
// auto-extract this from function signatures.

use serde::Serialize;
use std::collections::BTreeMap;

/// OpenAPI 3.0 specification builder.
pub struct OpenApiBuilder {
    info: ApiInfo,
    paths: BTreeMap<String, PathItem>,
    servers: Vec<Server>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiInfo {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Server {
    pub url: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Operation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Operation {
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "operationId")]
    pub operation_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub responses: BTreeMap<String, ResponseObj>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "requestBody")]
    pub request_body: Option<RequestBody>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseObj {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<BTreeMap<String, MediaType>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaType {
    pub schema: SchemaRef,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaRef {
    #[serde(rename = "type")]
    pub schema_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestBody {
    pub required: bool,
    pub content: BTreeMap<String, MediaType>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String, // path, query, header
    pub required: bool,
    pub schema: SchemaRef,
}

impl OpenApiBuilder {
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            info: ApiInfo {
                title: title.into(),
                version: version.into(),
                description: None,
            },
            paths: BTreeMap::new(),
            servers: Vec::new(),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.info.description = Some(desc.into());
        self
    }

    pub fn server(mut self, url: impl Into<String>, desc: Option<String>) -> Self {
        self.servers.push(Server {
            url: url.into(),
            description: desc,
        });
        self
    }

    /// Register a GET endpoint.
    pub fn get(mut self, path: &str, summary: &str, operation_id: &str) -> Self {
        let item = self.paths.entry(path.to_string()).or_default();
        item.get = Some(Operation {
            summary: summary.to_string(),
            description: None,
            operation_id: operation_id.to_string(),
            tags: Vec::new(),
            responses: default_responses(),
            request_body: None,
            parameters: Vec::new(),
        });
        self
    }

    /// Register a POST endpoint with JSON body.
    pub fn post(mut self, path: &str, summary: &str, operation_id: &str) -> Self {
        let item = self.paths.entry(path.to_string()).or_default();
        item.post = Some(Operation {
            summary: summary.to_string(),
            description: None,
            operation_id: operation_id.to_string(),
            tags: Vec::new(),
            responses: default_responses(),
            request_body: Some(RequestBody {
                required: true,
                content: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        "application/json".to_string(),
                        MediaType {
                            schema: SchemaRef {
                                schema_type: "object".to_string(),
                            },
                        },
                    );
                    m
                },
            }),
            parameters: Vec::new(),
        });
        self
    }

    /// Register a PUT endpoint.
    pub fn put(mut self, path: &str, summary: &str, operation_id: &str) -> Self {
        let item = self.paths.entry(path.to_string()).or_default();
        item.put = Some(Operation {
            summary: summary.to_string(),
            description: None,
            operation_id: operation_id.to_string(),
            tags: Vec::new(),
            responses: default_responses(),
            request_body: Some(RequestBody {
                required: true,
                content: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        "application/json".to_string(),
                        MediaType {
                            schema: SchemaRef {
                                schema_type: "object".to_string(),
                            },
                        },
                    );
                    m
                },
            }),
            parameters: Vec::new(),
        });
        self
    }

    /// Register a DELETE endpoint.
    pub fn delete(mut self, path: &str, summary: &str, operation_id: &str) -> Self {
        let item = self.paths.entry(path.to_string()).or_default();
        item.delete = Some(Operation {
            summary: summary.to_string(),
            description: None,
            operation_id: operation_id.to_string(),
            tags: Vec::new(),
            responses: default_responses(),
            request_body: None,
            parameters: Vec::new(),
        });
        self
    }

    /// Build the OpenAPI JSON specification.
    pub fn build_json(&self) -> String {
        let spec = serde_json::json!({
            "openapi": "3.0.3",
            "info": self.info,
            "servers": self.servers,
            "paths": self.paths,
        });
        serde_json::to_string_pretty(&spec).unwrap_or_default()
    }
}

fn default_responses() -> BTreeMap<String, ResponseObj> {
    let mut m = BTreeMap::new();
    m.insert(
        "200".to_string(),
        ResponseObj {
            description: "Successful response".to_string(),
            content: Some({
                let mut c = BTreeMap::new();
                c.insert(
                    "application/json".to_string(),
                    MediaType {
                        schema: SchemaRef {
                            schema_type: "object".to_string(),
                        },
                    },
                );
                c
            }),
        },
    );
    m
}

/// Create an Axum handler that serves the OpenAPI spec.
pub fn openapi_handler(
    spec_json: String,
) -> impl Fn() -> std::pin::Pin<
    Box<
        dyn std::future::Future<
                Output = (
                    axum::http::StatusCode,
                    [(axum::http::header::HeaderName, &'static str); 1],
                    String,
                ),
            > + Send,
    >,
> + Clone {
    move || {
        let json = spec_json.clone();
        Box::pin(async move {
            (
                axum::http::StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                json,
            )
        })
    }
}

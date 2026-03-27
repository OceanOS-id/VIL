#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpFormat {
    SSE,
    NDJSON,
    Raw,
}

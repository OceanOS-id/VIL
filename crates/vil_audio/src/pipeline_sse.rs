//! SSE pipeline builders for audio transcription operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for audio transcription streaming via `vil_workflow!`.

use vil_sdk::prelude::*;

/// Build an HTTP sink that accepts audio transcription requests via POST.
pub fn audio_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("AudioSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source for audio transcription streaming.
pub fn audio_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("AudioSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("text")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}

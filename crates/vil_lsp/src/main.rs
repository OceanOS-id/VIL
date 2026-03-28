use tower_lsp::{LspService, Server};
use tracing_subscriber::EnvFilter;

mod backend;
mod completions;
mod diagnostics;
mod hover;
mod parser;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(backend::VilBackend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

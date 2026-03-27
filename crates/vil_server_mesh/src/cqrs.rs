// =============================================================================
// VIL Server Mesh — CQRS (Command/Query Responsibility Segregation)
// =============================================================================
//
// Separates write operations (commands) from read operations (queries).
// Commands go through the Trigger Lane, queries through the Data Lane.
//
// Benefits:
//   - Independent scaling of read vs write paths
//   - Write path can go through validation pipeline
//   - Read path can be cached aggressively
//   - Different SHM region sizes for each path

use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;

/// A command (write operation).
pub trait Command: Serialize + DeserializeOwned + Send + Sync {
    /// Command name for routing.
    fn command_name(&self) -> &str;
}

/// A query (read operation).
pub trait Query: Serialize + DeserializeOwned + Send + Sync {
    /// Query name for routing.
    fn query_name(&self) -> &str;
    /// Result type name (for cache keying).
    fn result_type(&self) -> &str;
}

/// Command handler trait.
pub trait CommandHandler: Send + Sync {
    fn handle(&self, input: &[u8]) -> Result<Vec<u8>, String>;
    fn command_name(&self) -> &str;
}

/// Query handler trait.
pub trait QueryHandler: Send + Sync {
    fn handle(&self, input: &[u8]) -> Result<Vec<u8>, String>;
    fn query_name(&self) -> &str;
}

/// CQRS dispatcher — routes commands and queries to their handlers.
pub struct CqrsDispatcher {
    command_handlers: HashMap<String, Box<dyn CommandHandler>>,
    query_handlers: HashMap<String, Box<dyn QueryHandler>>,
}

impl CqrsDispatcher {
    pub fn new() -> Self {
        Self {
            command_handlers: HashMap::new(),
            query_handlers: HashMap::new(),
        }
    }

    /// Register a command handler.
    pub fn register_command<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&[u8]) -> Result<Vec<u8>, String> + Send + Sync + 'static,
    {
        struct FnHandler {
            name: String,
            f: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, String> + Send + Sync>,
        }
        impl CommandHandler for FnHandler {
            fn handle(&self, input: &[u8]) -> Result<Vec<u8>, String> { (self.f)(input) }
            fn command_name(&self) -> &str { &self.name }
        }
        self.command_handlers.insert(name.to_string(), Box::new(FnHandler {
            name: name.to_string(),
            f: Box::new(handler),
        }));
    }

    /// Register a query handler.
    pub fn register_query<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&[u8]) -> Result<Vec<u8>, String> + Send + Sync + 'static,
    {
        struct FnHandler {
            name: String,
            f: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, String> + Send + Sync>,
        }
        impl QueryHandler for FnHandler {
            fn handle(&self, input: &[u8]) -> Result<Vec<u8>, String> { (self.f)(input) }
            fn query_name(&self) -> &str { &self.name }
        }
        self.query_handlers.insert(name.to_string(), Box::new(FnHandler {
            name: name.to_string(),
            f: Box::new(handler),
        }));
    }

    /// Dispatch a command.
    pub fn dispatch_command(&self, name: &str, input: &[u8]) -> Result<Vec<u8>, String> {
        let handler = self.command_handlers.get(name)
            .ok_or_else(|| format!("Command '{}' not found", name))?;
        handler.handle(input)
    }

    /// Dispatch a query.
    pub fn dispatch_query(&self, name: &str, input: &[u8]) -> Result<Vec<u8>, String> {
        let handler = self.query_handlers.get(name)
            .ok_or_else(|| format!("Query '{}' not found", name))?;
        handler.handle(input)
    }

    /// List registered commands.
    pub fn commands(&self) -> Vec<String> {
        self.command_handlers.keys().cloned().collect()
    }

    /// List registered queries.
    pub fn queries(&self) -> Vec<String> {
        self.query_handlers.keys().cloned().collect()
    }
}

impl Default for CqrsDispatcher {
    fn default() -> Self { Self::new() }
}

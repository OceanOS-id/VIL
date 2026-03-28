// =============================================================================
// VIL GraphQL Plugin
// =============================================================================
//
// Auto-generates GraphQL schema from VilEntityMeta.
// CRUD resolvers delegate to DbProvider (1 vtable call).
// Subscriptions bridge EventBus → WebSocket.

pub mod axum_handler;
pub mod config;
pub mod filter;
pub mod pagination;
pub mod playground;
pub mod resolver;
pub mod schema;
pub mod subscription;

pub use config::GraphQLConfig;
pub use schema::VilSchemaBuilder;

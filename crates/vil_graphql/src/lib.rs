// =============================================================================
// VIL GraphQL Plugin
// =============================================================================
//
// Auto-generates GraphQL schema from VilEntityMeta.
// CRUD resolvers delegate to DbProvider (1 vtable call).
// Subscriptions bridge EventBus → WebSocket.

pub mod schema;
pub mod resolver;
pub mod filter;
pub mod pagination;
pub mod subscription;
pub mod playground;
pub mod config;
pub mod axum_handler;

pub use config::GraphQLConfig;
pub use schema::VilSchemaBuilder;

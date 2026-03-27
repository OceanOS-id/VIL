// =============================================================================
// VIL GraphQL — Schema Builder
// =============================================================================
//
// Builds an async-graphql schema from registered VilEntityMeta types.
// CRUD resolvers auto-generated: query, mutation, subscription.


use crate::config::GraphQLConfig;

/// Dynamic schema builder from VilEntityMeta.
pub struct VilSchemaBuilder {
    config: GraphQLConfig,
    entities: Vec<EntityDef>,
}

/// Entity definition for schema generation.
#[derive(Debug, Clone)]
pub struct EntityDef {
    pub name: String,
    pub table: String,
    pub source: String,
    pub primary_key: String,
    pub fields: Vec<FieldDef>,
}

/// Field definition.
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub graphql_type: String, // "Int!", "String!", "Float!", "Boolean!"
}

impl VilSchemaBuilder {
    pub fn new(config: GraphQLConfig) -> Self {
        Self { config, entities: Vec::new() }
    }

    /// Register an entity for schema generation.
    pub fn entity(mut self, def: EntityDef) -> Self {
        self.entities.push(def);
        self
    }

    /// Get registered entity count.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Get registered entity names.
    pub fn entity_names(&self) -> Vec<String> {
        self.entities.iter().map(|e| e.name.clone()).collect()
    }

    /// Get config.
    pub fn config(&self) -> &GraphQLConfig {
        &self.config
    }

    /// Build schema description (for introspection/documentation).
    pub fn describe_schema(&self) -> SchemaDescription {
        let mut types = Vec::new();
        let mut queries = Vec::new();
        let mut mutations = Vec::new();

        for entity in &self.entities {
            // Object type
            let fields: Vec<String> = entity.fields.iter()
                .map(|f| format!("{}: {}", f.name, f.graphql_type))
                .collect();
            types.push(format!("type {} {{ {} }}", entity.name, fields.join(", ")));

            // Queries
            queries.push(format!(
                "{}(id: Int!): {}",
                to_camel_case(&entity.name, false), entity.name
            ));
            queries.push(format!(
                "{}s(limit: Int, offset: Int): [{}!]!",
                to_camel_case(&entity.name, false), entity.name
            ));
            queries.push(format!(
                "{}Count: Int!",
                to_camel_case(&entity.name, false)
            ));

            // Mutations
            mutations.push(format!(
                "create{}(input: Create{}Input!): {}!",
                entity.name, entity.name, entity.name
            ));
            mutations.push(format!(
                "update{}(id: Int!, input: Update{}Input!): {}!",
                entity.name, entity.name, entity.name
            ));
            mutations.push(format!(
                "delete{}(id: Int!): Boolean!",
                entity.name
            ));
        }

        SchemaDescription { types, queries, mutations }
    }
}

/// Schema description for introspection.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SchemaDescription {
    pub types: Vec<String>,
    pub queries: Vec<String>,
    pub mutations: Vec<String>,
}

fn to_camel_case(s: &str, capitalize_first: bool) -> String {
    let mut result = String::new();
    let mut cap_next = capitalize_first;
    for ch in s.chars() {
        if ch == '_' {
            cap_next = true;
        } else if cap_next {
            result.push(ch.to_ascii_uppercase());
            cap_next = false;
        } else {
            result.push(ch.to_ascii_lowercase());
        }
    }
    result
}

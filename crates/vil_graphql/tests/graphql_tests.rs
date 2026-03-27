// =============================================================================
// V8 GraphQL — Unit Tests
// =============================================================================

// ==================== Config Tests ====================

#[test]
fn test_config_defaults() {
    use vil_graphql::GraphQLConfig;
    let config = GraphQLConfig::default();
    assert!(config.enabled);
    assert!(config.playground);
    assert!(config.introspection);
    assert_eq!(config.max_depth, 10);
    assert_eq!(config.max_complexity, 1000);
    assert_eq!(config.default_page_size, 20);
    assert_eq!(config.max_page_size, 100);
}

// ==================== Schema Builder Tests ====================

#[test]
fn test_schema_builder_empty() {
    use vil_graphql::VilSchemaBuilder;
    use vil_graphql::GraphQLConfig;

    let builder = VilSchemaBuilder::new(GraphQLConfig::default());
    assert_eq!(builder.entity_count(), 0);
    assert!(builder.entity_names().is_empty());
}

#[test]
fn test_schema_builder_register_entity() {
    use vil_graphql::VilSchemaBuilder;
    use vil_graphql::GraphQLConfig;
    use vil_graphql::schema::{EntityDef, FieldDef};

    let builder = VilSchemaBuilder::new(GraphQLConfig::default())
        .entity(EntityDef {
            name: "Order".into(),
            table: "orders".into(),
            source: "main_db".into(),
            primary_key: "id".into(),
            fields: vec![
                FieldDef { name: "id".into(), graphql_type: "Int!".into() },
                FieldDef { name: "total".into(), graphql_type: "Float!".into() },
                FieldDef { name: "status".into(), graphql_type: "String!".into() },
            ],
        })
        .entity(EntityDef {
            name: "Customer".into(),
            table: "customers".into(),
            source: "main_db".into(),
            primary_key: "id".into(),
            fields: vec![
                FieldDef { name: "id".into(), graphql_type: "Int!".into() },
                FieldDef { name: "name".into(), graphql_type: "String!".into() },
            ],
        });

    assert_eq!(builder.entity_count(), 2);
    assert!(builder.entity_names().contains(&"Order".to_string()));
    assert!(builder.entity_names().contains(&"Customer".to_string()));
}

#[test]
fn test_schema_description() {
    use vil_graphql::VilSchemaBuilder;
    use vil_graphql::GraphQLConfig;
    use vil_graphql::schema::{EntityDef, FieldDef};

    let builder = VilSchemaBuilder::new(GraphQLConfig::default())
        .entity(EntityDef {
            name: "Order".into(),
            table: "orders".into(),
            source: "main_db".into(),
            primary_key: "id".into(),
            fields: vec![
                FieldDef { name: "id".into(), graphql_type: "Int!".into() },
                FieldDef { name: "total".into(), graphql_type: "Float!".into() },
            ],
        });

    let desc = builder.describe_schema();
    assert_eq!(desc.types.len(), 1);
    assert!(desc.types[0].contains("Order"));
    assert!(desc.types[0].contains("id: Int!"));
    assert!(desc.types[0].contains("total: Float!"));

    // Should have queries: order, orders, orderCount
    assert_eq!(desc.queries.len(), 3);

    // Should have mutations: createOrder, updateOrder, deleteOrder
    assert_eq!(desc.mutations.len(), 3);
    assert!(desc.mutations.iter().any(|m| m.contains("createOrder")));
    assert!(desc.mutations.iter().any(|m| m.contains("deleteOrder")));
}

// ==================== Filter Tests ====================

#[test]
fn test_build_where_clause_empty() {
    use vil_graphql::filter::build_where_clause;

    let (clause, params) = build_where_clause(&serde_json::json!({}));
    assert_eq!(clause, "1=1");
    assert!(params.is_empty());
}

#[test]
fn test_build_where_clause_eq() {
    use vil_graphql::filter::build_where_clause;

    let (clause, params) = build_where_clause(&serde_json::json!({
        "status": { "eq": "active" }
    }));
    assert!(clause.contains("status = ?"));
    assert_eq!(params.len(), 1);
}

#[test]
fn test_build_where_clause_multiple() {
    use vil_graphql::filter::build_where_clause;

    let (clause, params) = build_where_clause(&serde_json::json!({
        "age": { "gt": 18 },
        "name": { "contains": "john" }
    }));
    assert!(clause.contains("AND"));
    assert_eq!(params.len(), 2);
}

#[test]
fn test_build_where_clause_lt_gt() {
    use vil_graphql::filter::build_where_clause;

    let (clause, params) = build_where_clause(&serde_json::json!({
        "price": { "gt": 10, "lt": 100 }
    }));
    assert!(clause.contains("price > ?"));
    assert!(clause.contains("price < ?"));
    assert_eq!(params.len(), 2);
}

// ==================== Pagination Tests ====================

#[test]
fn test_calc_pagination_defaults() {
    use vil_graphql::pagination::calc_pagination;

    let (size, offset) = calc_pagination(None, None, 20, 100);
    assert_eq!(size, 20);
    assert_eq!(offset, 0);
}

#[test]
fn test_calc_pagination_custom() {
    use vil_graphql::pagination::calc_pagination;

    let (size, offset) = calc_pagination(Some(50), Some(10), 20, 100);
    assert_eq!(size, 50);
    assert_eq!(offset, 10);
}

#[test]
fn test_calc_pagination_capped() {
    use vil_graphql::pagination::calc_pagination;

    let (size, _) = calc_pagination(Some(500), None, 20, 100);
    assert_eq!(size, 100); // Capped at max
}

// ==================== Subscription Tests ====================

#[test]
fn test_subscription_topic() {
    use vil_graphql::subscription::{entity_topic, SubscriptionOp};

    assert_eq!(entity_topic("Order", &SubscriptionOp::Created), "order:created");
    assert_eq!(entity_topic("Order", &SubscriptionOp::Updated), "order:updated");
    assert_eq!(entity_topic("Order", &SubscriptionOp::Deleted), "order:deleted");
}

#[test]
fn test_subscription_registry() {
    use vil_graphql::subscription::SubscriptionRegistry;

    let reg = SubscriptionRegistry::new();
    assert_eq!(reg.subscriber_count("order:created"), 0);
    assert!(reg.active_topics().is_empty());

    reg.subscribe("order:created");
    reg.subscribe("order:created");
    reg.subscribe("order:updated");

    assert_eq!(reg.subscriber_count("order:created"), 2);
    assert_eq!(reg.subscriber_count("order:updated"), 1);
    assert_eq!(reg.active_topics().len(), 2);

    reg.unsubscribe("order:created");
    assert_eq!(reg.subscriber_count("order:created"), 1);
}

// ==================== Resolver JSON Conversion Tests ====================

#[test]
fn test_resolver_json_to_sql_value() {
    use vil_graphql::resolver::CrudResolver;
    use vil_db_semantic::ToSqlValue;

    // Test via the public API indirectly — CrudResolver requires DbProvider
    // So we test the json conversion via filter module
    use vil_graphql::filter::build_where_clause;

    let (_, params) = build_where_clause(&serde_json::json!({
        "id": { "eq": 42 },
        "name": { "eq": "test" },
        "active": { "eq": true }
    }));

    assert_eq!(params.len(), 3);
}

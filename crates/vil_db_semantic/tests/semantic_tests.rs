// =============================================================================
// V7 DB Semantic Layer — Unit Tests
// =============================================================================

// ==================== DatasourceRef Tests ====================

#[test]
fn test_datasource_ref() {
    use vil_db_semantic::DatasourceRef;

    let ds = DatasourceRef::new("main_db");
    assert_eq!(ds.name(), "main_db");
    assert_eq!(ds.to_string(), "main_db");

    // Prove zero-cost: size_of DatasourceRef == size_of &str
    assert_eq!(
        std::mem::size_of::<DatasourceRef>(),
        std::mem::size_of::<&str>()
    );
}

#[test]
fn test_datasource_ref_const() {
    use vil_db_semantic::DatasourceRef;

    const MAIN: DatasourceRef = DatasourceRef::new("main_db");
    const AUDIT: DatasourceRef = DatasourceRef::new("audit_db");
    assert_ne!(MAIN, AUDIT);
    assert_eq!(MAIN.name(), "main_db");
}

// ==================== TxScope Tests ====================

#[test]
fn test_tx_scope_size() {
    use vil_db_semantic::TxScope;
    // Must be 1 byte
    assert_eq!(std::mem::size_of::<TxScope>(), 1);
}

#[test]
fn test_tx_scope_default() {
    use vil_db_semantic::TxScope;
    assert_eq!(TxScope::default(), TxScope::None);
}

#[test]
fn test_tx_scope_variants() {
    use vil_db_semantic::TxScope;
    let scopes = [TxScope::ReadOnly, TxScope::ReadWrite, TxScope::RequiresNew, TxScope::JoinIfPresent, TxScope::None];
    assert_eq!(scopes.len(), 5);
}

// ==================== DbCapability Tests ====================

#[test]
fn test_capability_bitflag() {
    use vil_db_semantic::DbCapability;

    let cap = DbCapability::BASIC_CRUD.union(DbCapability::TRANSACTIONS);
    assert!(cap.contains(DbCapability::BASIC_CRUD));
    assert!(cap.contains(DbCapability::TRANSACTIONS));
    assert!(!cap.contains(DbCapability::RELATIONS));
}

#[test]
fn test_capability_size() {
    use vil_db_semantic::DbCapability;
    // Must be 4 bytes (u32)
    assert_eq!(std::mem::size_of::<DbCapability>(), 4);
}

#[test]
fn test_capability_presets() {
    use vil_db_semantic::DbCapability;

    let sql = DbCapability::SQL_STANDARD;
    assert!(sql.contains(DbCapability::BASIC_CRUD));
    assert!(sql.contains(DbCapability::TRANSACTIONS));
    assert!(!sql.contains(DbCapability::MIGRATION));

    let orm = DbCapability::ORM_FULL;
    assert!(orm.contains(DbCapability::BASIC_CRUD));
    assert!(orm.contains(DbCapability::TRANSACTIONS));
    assert!(orm.contains(DbCapability::RELATIONS));
    assert!(orm.contains(DbCapability::MIGRATION));
}

#[test]
fn test_capability_display() {
    use vil_db_semantic::DbCapability;

    let cap = DbCapability::BASIC_CRUD.union(DbCapability::TRANSACTIONS);
    let display = cap.to_string();
    assert!(display.contains("BasicCrud"));
    assert!(display.contains("Transactions"));
}

#[test]
fn test_capability_none() {
    use vil_db_semantic::DbCapability;

    let none = DbCapability::NONE;
    assert!(!none.contains(DbCapability::BASIC_CRUD));
    assert_eq!(none.0, 0);
}

// ==================== PortabilityTier Tests ====================

#[test]
fn test_portability_tier_size() {
    use vil_db_semantic::PortabilityTier;
    assert_eq!(std::mem::size_of::<PortabilityTier>(), 1);
}

#[test]
fn test_portability_display() {
    use vil_db_semantic::PortabilityTier;

    assert!(PortabilityTier::P0.to_string().contains("Portable"));
    assert!(PortabilityTier::P1.to_string().contains("Capability"));
    assert!(PortabilityTier::P2.to_string().contains("Provider"));
}

// ==================== CachePolicy Tests ====================

#[test]
fn test_cache_policy_size() {
    use vil_db_semantic::CachePolicy;
    // Enum with u32 variant = 8 bytes (discriminant + u32)
    assert!(std::mem::size_of::<CachePolicy>() <= 8);
}

#[test]
fn test_cache_policy_default() {
    use vil_db_semantic::CachePolicy;
    assert_eq!(CachePolicy::default(), CachePolicy::None);
}

// ==================== VilEntityMeta Tests ====================

#[test]
fn test_entity_meta_manual_impl() {
    use vil_db_semantic::VilEntityMeta;
    use vil_db_semantic::PortabilityTier;

    struct Order;
    impl VilEntityMeta for Order {
        const TABLE: &'static str = "orders";
        const SOURCE: &'static str = "main_db";
        const PRIMARY_KEY: &'static str = "id";
        const FIELDS: &'static [&'static str] = &["id", "customer_id", "total"];
    }

    assert_eq!(Order::TABLE, "orders");
    assert_eq!(Order::SOURCE, "main_db");
    assert_eq!(Order::PRIMARY_KEY, "id");
    assert_eq!(Order::FIELDS.len(), 3);
    assert_eq!(Order::PORTABILITY, PortabilityTier::P0); // default
}

// ==================== DbError Tests ====================

#[test]
fn test_db_error_display() {
    use vil_db_semantic::error::DbError;

    let err = DbError::NotFound;
    assert_eq!(err.to_string(), "Entity not found");

    let err = DbError::CapabilityMissing("BULK_INSERT".into());
    assert!(err.to_string().contains("BULK_INSERT"));
}

// ==================== ToSqlValue Tests ====================

#[test]
fn test_to_sql_value() {
    use vil_db_semantic::ToSqlValue;

    let v1 = ToSqlValue::Int(42);
    let v2 = ToSqlValue::Text("hello".into());
    let v3 = ToSqlValue::Null;
    let v4 = ToSqlValue::Bool(true);
    let v5 = ToSqlValue::Float(3.14);

    // Just ensure they construct without panic
    assert!(matches!(v1, ToSqlValue::Int(42)));
    assert!(matches!(v3, ToSqlValue::Null));
}

// ==================== DatasourceRegistry Tests ====================

#[tokio::test]
async fn test_registry_resolve_not_found() {
    use vil_db_semantic::DatasourceRegistry;

    let registry = DatasourceRegistry::new();
    let result = registry.resolve("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_registry_empty() {
    use vil_db_semantic::DatasourceRegistry;

    let registry = DatasourceRegistry::new();
    assert_eq!(registry.count(), 0);
    assert!(registry.list().is_empty());
}

// ==================== Zero-Cost Proof Tests ====================

#[test]
fn test_zero_cost_proof() {
    use vil_db_semantic::*;

    // Prove all semantic primitives are stack-allocated and tiny
    assert_eq!(std::mem::size_of::<DatasourceRef>(), 16); // &'static str = ptr + len
    assert_eq!(std::mem::size_of::<TxScope>(), 1);
    assert_eq!(std::mem::size_of::<DbCapability>(), 4);
    assert_eq!(std::mem::size_of::<PortabilityTier>(), 1);
    assert!(std::mem::size_of::<CachePolicy>() <= 8);

    // Total stack footprint for all semantic context:
    let total = std::mem::size_of::<DatasourceRef>()
        + std::mem::size_of::<TxScope>()
        + std::mem::size_of::<DbCapability>()
        + std::mem::size_of::<PortabilityTier>()
        + std::mem::size_of::<CachePolicy>();

    // All semantic context fits in 30 bytes — less than a cache line (64 bytes)
    assert!(total <= 64, "Semantic context must fit in one cache line, got {} bytes", total);
}

// u32 bitflag — stack-allocated, compile-time checkable.

/// Database provider capabilities (bitflag).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DbCapability(pub u32);

impl DbCapability {
    pub const BASIC_CRUD: Self = Self(1 << 0);
    pub const TRANSACTIONS: Self = Self(1 << 1);
    pub const RELATIONS: Self = Self(1 << 2);
    pub const BULK_INSERT: Self = Self(1 << 3);
    pub const STREAMING_CURSOR: Self = Self(1 << 4);
    pub const JSON_QUERY: Self = Self(1 << 5);
    pub const FULL_TEXT_SEARCH: Self = Self(1 << 6);
    pub const NESTED_TX: Self = Self(1 << 7);
    pub const REPLICA_READ: Self = Self(1 << 8);
    pub const MIGRATION: Self = Self(1 << 9);
    pub const CACHE_KV: Self = Self(1 << 10);
    pub const PUBSUB: Self = Self(1 << 11);

    pub const NONE: Self = Self(0);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Standard SQL provider (CRUD + TX)
    pub const SQL_STANDARD: Self = Self(Self::BASIC_CRUD.0 | Self::TRANSACTIONS.0);

    /// Full ORM provider (CRUD + TX + Relations + Migration)
    pub const ORM_FULL: Self =
        Self(Self::BASIC_CRUD.0 | Self::TRANSACTIONS.0 | Self::RELATIONS.0 | Self::MIGRATION.0);
}

impl std::fmt::Display for DbCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut caps = Vec::new();
        if self.contains(Self::BASIC_CRUD) {
            caps.push("BasicCrud");
        }
        if self.contains(Self::TRANSACTIONS) {
            caps.push("Transactions");
        }
        if self.contains(Self::RELATIONS) {
            caps.push("Relations");
        }
        if self.contains(Self::BULK_INSERT) {
            caps.push("BulkInsert");
        }
        if self.contains(Self::STREAMING_CURSOR) {
            caps.push("StreamingCursor");
        }
        if self.contains(Self::JSON_QUERY) {
            caps.push("JsonQuery");
        }
        if self.contains(Self::FULL_TEXT_SEARCH) {
            caps.push("FullTextSearch");
        }
        if self.contains(Self::NESTED_TX) {
            caps.push("NestedTx");
        }
        if self.contains(Self::REPLICA_READ) {
            caps.push("ReplicaRead");
        }
        if self.contains(Self::MIGRATION) {
            caps.push("Migration");
        }
        write!(f, "[{}]", caps.join(", "))
    }
}

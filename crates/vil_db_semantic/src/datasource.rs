// Zero heap — &'static str alias to configured datasource.

/// Reference to a configured datasource. Zero allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DatasourceRef(pub &'static str);

impl DatasourceRef {
    pub const fn new(name: &'static str) -> Self { Self(name) }
    pub fn name(&self) -> &'static str { self.0 }
}

impl std::fmt::Display for DatasourceRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

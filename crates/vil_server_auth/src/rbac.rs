// =============================================================================
// VIL Server Auth — Role-Based Access Control (RBAC)
// =============================================================================
//
// Provides fine-grained access control based on roles and permissions.
// Integrates with JWT claims (roles field) and API key scopes.
//
// Model:
//   User → has Roles → Roles have Permissions → Permissions gate Routes
//
// Example:
//   Role "admin" → permissions: ["users:read", "users:write", "orders:*"]
//   Role "viewer" → permissions: ["users:read", "orders:read"]
//
// Route protection:
//   .route("/admin/users", get(list_users).layer(require_permission("users:read")))

use dashmap::DashMap;
use std::collections::HashSet;

/// Permission string (e.g., "users:read", "orders:write", "admin:*").
pub type Permission = String;

/// Role definition with associated permissions.
#[derive(Debug, Clone)]
pub struct Role {
    pub name: String,
    pub permissions: HashSet<Permission>,
    pub description: String,
}

impl Role {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            permissions: HashSet::new(),
            description: String::new(),
        }
    }

    pub fn permission(mut self, perm: impl Into<String>) -> Self {
        self.permissions.insert(perm.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Check if this role has a specific permission.
    /// Supports wildcard: "admin:*" matches "admin:read", "admin:write", etc.
    pub fn has_permission(&self, required: &str) -> bool {
        if self.permissions.contains(required) {
            return true;
        }
        // Check wildcard patterns
        let parts: Vec<&str> = required.split(':').collect();
        if parts.len() == 2 {
            let wildcard = format!("{}:*", parts[0]);
            if self.permissions.contains(&wildcard) {
                return true;
            }
        }
        // Check global wildcard
        self.permissions.contains("*")
    }
}

/// RBAC policy store.
pub struct RbacPolicy {
    /// Defined roles
    roles: DashMap<String, Role>,
}

impl RbacPolicy {
    pub fn new() -> Self {
        Self {
            roles: DashMap::new(),
        }
    }

    /// Define a role.
    pub fn add_role(&self, role: Role) {
        self.roles.insert(role.name.clone(), role);
    }

    /// Check if a set of role names has the required permission.
    pub fn check_permission(&self, user_roles: &[String], required: &str) -> bool {
        for role_name in user_roles {
            if let Some(role) = self.roles.get(role_name) {
                if role.has_permission(required) {
                    return true;
                }
            }
        }
        false
    }

    /// Get all permissions for a set of roles.
    pub fn effective_permissions(&self, user_roles: &[String]) -> HashSet<Permission> {
        let mut perms = HashSet::new();
        for role_name in user_roles {
            if let Some(role) = self.roles.get(role_name) {
                perms.extend(role.permissions.clone());
            }
        }
        perms
    }

    /// List all defined roles.
    pub fn list_roles(&self) -> Vec<String> {
        self.roles.iter().map(|e| e.key().clone()).collect()
    }

    /// Get a role by name.
    pub fn get_role(&self, name: &str) -> Option<Role> {
        self.roles.get(name).map(|r| r.clone())
    }
}

impl Default for RbacPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// RBAC status for debugging/admin.
#[derive(Debug, serde::Serialize)]
pub struct RbacStatus {
    pub roles_defined: usize,
    pub role_names: Vec<String>,
}

impl RbacPolicy {
    pub fn status(&self) -> RbacStatus {
        RbacStatus {
            roles_defined: self.roles.len(),
            role_names: self.list_roles(),
        }
    }
}

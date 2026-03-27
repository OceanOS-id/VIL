// =============================================================================
// VIL Server Auth — Unit Tests
// =============================================================================

// ==================== API Key Tests ====================

#[cfg(test)]
mod api_key_tests {
    use vil_server_auth::api_key::ApiKeyAuth;

    #[test]
    fn test_add_and_validate_key() {
        let auth = ApiKeyAuth::new();
        auth.add_key("sk-test-123", "Test App");

        let info = auth.validate("sk-test-123");
        assert!(info.is_some());
        assert_eq!(info.unwrap().name, "Test App");
    }

    #[test]
    fn test_invalid_key() {
        let auth = ApiKeyAuth::new();
        auth.add_key("valid-key", "App");
        assert!(auth.validate("wrong-key").is_none());
    }

    #[test]
    fn test_revoke_key() {
        let auth = ApiKeyAuth::new();
        auth.add_key("key1", "App");
        assert!(auth.validate("key1").is_some());

        auth.revoke_key("key1");
        assert!(auth.validate("key1").is_none());
    }

    #[test]
    fn test_scoped_key() {
        let auth = ApiKeyAuth::new();
        auth.add_key_scoped("key1", "App", vec!["read".to_string(), "write".to_string()]);

        let info = auth.validate("key1").unwrap();
        assert_eq!(info.scopes.len(), 2);
        assert!(info.scopes.contains(&"read".to_string()));
    }

    #[test]
    fn test_key_count() {
        let auth = ApiKeyAuth::new();
        auth.add_key("k1", "a");
        auth.add_key("k2", "b");
        assert_eq!(auth.key_count(), 2);
    }
}

// ==================== IP Filter Tests ====================

#[cfg(test)]
mod ip_filter_tests {
    use vil_server_auth::ip_filter::{IpFilter, IpFilterMode};
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_allowlist_allows_listed() {
        let filter = IpFilter::allowlist()
            .add_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));

        assert!(filter.is_allowed(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(!filter.is_allowed(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2))));
    }

    #[test]
    fn test_blocklist_blocks_listed() {
        let filter = IpFilter::blocklist()
            .add_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)));

        assert!(!filter.is_allowed(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))));
        assert!(filter.is_allowed(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101))));
    }

    #[test]
    fn test_cidr_matching() {
        let filter = IpFilter::allowlist()
            .add_cidr("10.0.0.0/8");

        assert!(filter.is_allowed(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(filter.is_allowed(&IpAddr::V4(Ipv4Addr::new(10, 255, 255, 255))));
        assert!(!filter.is_allowed(&IpAddr::V4(Ipv4Addr::new(11, 0, 0, 1))));
    }

    #[test]
    fn test_cidr_24() {
        let filter = IpFilter::allowlist()
            .add_cidr("192.168.1.0/24");

        assert!(filter.is_allowed(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 0))));
        assert!(filter.is_allowed(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 255))));
        assert!(!filter.is_allowed(&IpAddr::V4(Ipv4Addr::new(192, 168, 2, 1))));
    }

    #[test]
    fn test_filter_mode() {
        let allow = IpFilter::allowlist();
        assert_eq!(allow.mode(), IpFilterMode::Allowlist);

        let block = IpFilter::blocklist();
        assert_eq!(block.mode(), IpFilterMode::Blocklist);
    }
}

// ==================== RBAC Tests ====================

#[cfg(test)]
mod rbac_tests {
    use vil_server_auth::rbac::{RbacPolicy, Role};

    #[test]
    fn test_role_permission() {
        let role = Role::new("admin")
            .permission("users:read")
            .permission("users:write")
            .permission("orders:*");

        assert!(role.has_permission("users:read"));
        assert!(role.has_permission("users:write"));
        assert!(role.has_permission("orders:read")); // wildcard
        assert!(role.has_permission("orders:delete")); // wildcard
        assert!(!role.has_permission("settings:write"));
    }

    #[test]
    fn test_global_wildcard() {
        let role = Role::new("superadmin").permission("*");
        assert!(role.has_permission("anything:here"));
        assert!(role.has_permission("whatever"));
    }

    #[test]
    fn test_policy_check() {
        let policy = RbacPolicy::new();
        policy.add_role(Role::new("viewer").permission("users:read"));
        policy.add_role(Role::new("editor").permission("users:read").permission("users:write"));

        let viewer_roles = vec!["viewer".to_string()];
        assert!(policy.check_permission(&viewer_roles, "users:read"));
        assert!(!policy.check_permission(&viewer_roles, "users:write"));

        let editor_roles = vec!["editor".to_string()];
        assert!(policy.check_permission(&editor_roles, "users:read"));
        assert!(policy.check_permission(&editor_roles, "users:write"));
    }

    #[test]
    fn test_effective_permissions() {
        let policy = RbacPolicy::new();
        policy.add_role(Role::new("a").permission("x").permission("y"));
        policy.add_role(Role::new("b").permission("y").permission("z"));

        let roles = vec!["a".to_string(), "b".to_string()];
        let perms = policy.effective_permissions(&roles);
        assert!(perms.contains("x"));
        assert!(perms.contains("y"));
        assert!(perms.contains("z"));
        assert_eq!(perms.len(), 3);
    }

    #[test]
    fn test_list_roles() {
        let policy = RbacPolicy::new();
        policy.add_role(Role::new("admin"));
        policy.add_role(Role::new("user"));
        let roles = policy.list_roles();
        assert_eq!(roles.len(), 2);
    }
}

// ==================== CSRF Tests ====================

#[cfg(test)]
mod csrf_tests {
    use vil_server_auth::csrf::{CsrfConfig, CsrfProtection};
    use axum::http::{HeaderMap, HeaderValue, Method};

    #[test]
    fn test_generate_token() {
        let csrf = CsrfProtection::new(CsrfConfig::default());
        let token = csrf.generate_token();
        assert!(!token.is_empty());
        assert_eq!(token.len(), 64); // 32 bytes hex-encoded
    }

    #[test]
    fn test_safe_methods_exempt() {
        let csrf = CsrfProtection::new(CsrfConfig::default());
        assert!(!csrf.needs_check(&Method::GET, "/"));
        assert!(!csrf.needs_check(&Method::HEAD, "/"));
        assert!(!csrf.needs_check(&Method::OPTIONS, "/"));
        assert!(csrf.needs_check(&Method::POST, "/"));
        assert!(csrf.needs_check(&Method::PUT, "/"));
        assert!(csrf.needs_check(&Method::DELETE, "/"));
    }

    #[test]
    fn test_exempt_path() {
        let config = CsrfConfig::new().exempt_path("/api/webhook");
        let csrf = CsrfProtection::new(config);
        assert!(!csrf.needs_check(&Method::POST, "/api/webhook/github"));
        assert!(csrf.needs_check(&Method::POST, "/api/users"));
    }

    #[test]
    fn test_validate_matching_tokens() {
        let csrf = CsrfProtection::new(CsrfConfig::default());
        let token = "abc123def456";
        let mut headers = HeaderMap::new();
        headers.insert("x-csrf-token", HeaderValue::from_static("abc123def456"));
        assert!(csrf.validate(&headers, Some(token)));
    }

    #[test]
    fn test_validate_mismatching_tokens() {
        let csrf = CsrfProtection::new(CsrfConfig::default());
        let mut headers = HeaderMap::new();
        headers.insert("x-csrf-token", HeaderValue::from_static("wrong"));
        assert!(!csrf.validate(&headers, Some("correct")));
    }

    #[test]
    fn test_validate_missing_header() {
        let csrf = CsrfProtection::new(CsrfConfig::default());
        let headers = HeaderMap::new();
        assert!(!csrf.validate(&headers, Some("token")));
    }
}

// ==================== Session Tests ====================

#[cfg(test)]
mod session_tests {
    use vil_server_auth::session::{SessionConfig, SessionData, SessionManager};
    use std::time::Duration;

    #[test]
    fn test_create_session() {
        let mgr = SessionManager::default();
        let (id, data) = mgr.create();
        assert!(!id.is_empty());
        assert!(data.values.is_empty());
        assert_eq!(mgr.active_count(), 1);
    }

    #[test]
    fn test_get_session() {
        let mgr = SessionManager::default();
        let (id, _) = mgr.create();
        let data = mgr.get(&id);
        assert!(data.is_some());
    }

    #[test]
    fn test_update_session() {
        let mgr = SessionManager::default();
        let (id, _) = mgr.create();

        let mut data = SessionData::new();
        data.set("user_id", serde_json::json!(42));
        assert!(mgr.update(&id, data));

        let retrieved = mgr.get(&id).unwrap();
        assert_eq!(retrieved.get("user_id"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_destroy_session() {
        let mgr = SessionManager::default();
        let (id, _) = mgr.create();
        mgr.destroy(&id);
        assert!(mgr.get(&id).is_none());
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn test_session_ttl_expiry() {
        let mgr = SessionManager::new(SessionConfig {
            ttl: Duration::from_millis(10),
            ..Default::default()
        });
        let (id, _) = mgr.create();
        std::thread::sleep(Duration::from_millis(20));
        assert!(mgr.get(&id).is_none());
    }

    #[test]
    fn test_cleanup_expired() {
        let mgr = SessionManager::new(SessionConfig {
            ttl: Duration::from_millis(10),
            ..Default::default()
        });
        for _ in 0..5 {
            mgr.create();
        }
        std::thread::sleep(Duration::from_millis(20));
        let cleaned = mgr.cleanup_expired();
        assert_eq!(cleaned, 5);
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn test_cookie_header() {
        let mgr = SessionManager::default();
        let header = mgr.cookie_header("test-session-id");
        assert!(header.contains("vil-session=test-session-id"));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("SameSite=Lax"));
    }

    #[test]
    fn test_session_data_operations() {
        let mut data = SessionData::new();
        data.set("key", serde_json::json!("value"));
        assert!(data.contains("key"));
        assert_eq!(data.get("key"), Some(&serde_json::json!("value")));
        let removed = data.remove("key");
        assert_eq!(removed, Some(serde_json::json!("value")));
        assert!(!data.contains("key"));
    }
}

// ==================== Audit Log Tests ====================

#[cfg(test)]
mod audit_tests {
    use vil_server_auth::audit::{AuditLog, AuditEvent, AuditEventType};

    #[test]
    fn test_record_and_retrieve() {
        let log = AuditLog::new(100);
        let event = AuditEvent::new(AuditEventType::AuthSuccess, "user@test.com", "/login", "POST")
            .ip("10.0.0.1")
            .request_id("req-123");

        log.record(event);
        assert_eq!(log.count(), 1);

        let recent = log.recent(10);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].actor, "user@test.com");
    }

    #[test]
    fn test_ring_buffer_eviction() {
        let log = AuditLog::new(3);
        for i in 0..5 {
            log.record(AuditEvent::new(AuditEventType::AuthSuccess, &format!("user_{}", i), "/", "GET"));
        }
        assert_eq!(log.count(), 3); // max size
    }

    #[test]
    fn test_clear() {
        let log = AuditLog::new(100);
        log.record(AuditEvent::new(AuditEventType::AuthFailure, "bad_actor", "/login", "POST"));
        log.clear();
        assert_eq!(log.count(), 0);
    }
}

// ==================== Circuit Breaker Extended Tests ====================

#[cfg(test)]
mod circuit_breaker_extended_tests {
    use vil_server_auth::circuit_breaker::*;
    use std::time::Duration;

    #[test]
    fn test_status_export() {
        let cb = CircuitBreaker::new("test-svc", CircuitBreakerConfig::default());
        let status = cb.status();
        assert_eq!(status.service, "test-svc");
        assert_eq!(status.state, "Closed");
        assert_eq!(status.failures, 0);
        assert_eq!(status.successes, 0);
    }

    #[test]
    fn test_half_open_recovery() {
        let cb = CircuitBreaker::new("svc", CircuitBreakerConfig {
            failure_threshold: 2,
            cooldown: Duration::from_millis(10),
            half_open_max_requests: 1,
            ..Default::default()
        });

        // Trip the breaker
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for cooldown
        std::thread::sleep(Duration::from_millis(20));

        // Should transition to HalfOpen on check
        assert!(cb.check().is_ok());
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Successful request → Closed
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_failure() {
        let cb = CircuitBreaker::new("svc", CircuitBreakerConfig {
            failure_threshold: 1,
            cooldown: Duration::from_millis(10),
            half_open_max_requests: 1,
            ..Default::default()
        });

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        std::thread::sleep(Duration::from_millis(20));
        let _ = cb.check(); // → HalfOpen

        // Failure in HalfOpen → back to Open
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }
}

// =============================================================================
// VIL Server Core — Unit Tests
// =============================================================================

// ==================== Cache Tests ====================

#[cfg(test)]
mod cache_tests {
    use std::time::Duration;
    use vil_server_core::cache::Cache;

    #[test]
    fn test_cache_put_get() {
        let cache: Cache<String, String> = Cache::new(100, Duration::from_secs(60));
        cache.put("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
    }

    #[test]
    fn test_cache_miss() {
        let cache: Cache<String, String> = Cache::new(100, Duration::from_secs(60));
        assert_eq!(cache.get(&"nonexistent".to_string()), None);
    }

    #[test]
    fn test_cache_ttl_expiry() {
        let cache: Cache<String, String> = Cache::new(100, Duration::from_millis(10));
        cache.put("key1".to_string(), "value1".to_string());
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache: Cache<u32, u32> = Cache::new(3, Duration::from_secs(60));
        cache.put(1, 10);
        cache.put(2, 20);
        cache.put(3, 30);
        // Access key 1 to make it recently used
        let _ = cache.get(&1);
        // Adding key 4 should evict key 2 (LRU)
        cache.put(4, 40);
        assert_eq!(cache.get(&1), Some(10)); // recently accessed
        assert_eq!(cache.get(&3), Some(30));
        assert_eq!(cache.get(&4), Some(40));
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_cache_remove() {
        let cache: Cache<String, i32> = Cache::new(100, Duration::from_secs(60));
        cache.put("a".to_string(), 1);
        assert_eq!(cache.remove(&"a".to_string()), Some(1));
        assert_eq!(cache.get(&"a".to_string()), None);
    }

    #[test]
    fn test_cache_contains() {
        let cache: Cache<String, i32> = Cache::new(100, Duration::from_secs(60));
        cache.put("x".to_string(), 42);
        assert!(cache.contains(&"x".to_string()));
        assert!(!cache.contains(&"y".to_string()));
    }

    #[test]
    fn test_cache_clear() {
        let cache: Cache<u32, u32> = Cache::new(100, Duration::from_secs(60));
        for i in 0..10 {
            cache.put(i, i * 10);
        }
        assert_eq!(cache.len(), 10);
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_stats() {
        let cache: Cache<String, i32> = Cache::new(100, Duration::from_secs(60));
        cache.put("a".to_string(), 1);
        let _ = cache.get(&"a".to_string()); // hit
        let _ = cache.get(&"b".to_string()); // miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_cache_cleanup_expired() {
        let cache: Cache<u32, u32> = Cache::new(100, Duration::from_millis(10));
        for i in 0..5 {
            cache.put(i, i);
        }
        std::thread::sleep(Duration::from_millis(20));
        let evicted = cache.cleanup_expired();
        assert_eq!(evicted, 5);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_custom_ttl() {
        let cache: Cache<String, i32> = Cache::new(100, Duration::from_secs(60));
        cache.put_with_ttl("short".to_string(), 1, Duration::from_millis(10));
        cache.put_with_ttl("long".to_string(), 2, Duration::from_secs(60));
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(cache.get(&"short".to_string()), None); // expired
        assert_eq!(cache.get(&"long".to_string()), Some(2)); // still alive
    }
}

// ==================== ETag Tests ====================

#[cfg(test)]
mod etag_tests {
    use vil_server_core::etag::*;

    #[test]
    fn test_generate_etag() {
        let etag = generate_etag(b"hello world");
        assert!(etag.starts_with("W/\""));
        assert!(etag.ends_with("\""));
    }

    #[test]
    fn test_etag_deterministic() {
        let e1 = generate_etag(b"same content");
        let e2 = generate_etag(b"same content");
        assert_eq!(e1, e2);
    }

    #[test]
    fn test_etag_different_content() {
        let e1 = generate_etag(b"content a");
        let e2 = generate_etag(b"content b");
        assert_ne!(e1, e2);
    }

    #[test]
    fn test_matches_etag_exact() {
        let etag = generate_etag(b"test");
        assert!(matches_etag(Some(&etag), &etag));
    }

    #[test]
    fn test_matches_etag_wildcard() {
        let etag = generate_etag(b"test");
        assert!(matches_etag(Some("*"), &etag));
    }

    #[test]
    fn test_matches_etag_no_match() {
        let etag = generate_etag(b"test");
        assert!(!matches_etag(Some("W/\"different\""), &etag));
    }

    #[test]
    fn test_matches_etag_none() {
        let etag = generate_etag(b"test");
        assert!(!matches_etag(None, &etag));
    }
}

// ==================== Retry Tests ====================

#[cfg(test)]
mod retry_tests {
    use std::time::Duration;
    use vil_server_core::retry::*;

    #[test]
    fn test_fixed_delay() {
        let strategy = RetryStrategy::Fixed {
            delay: Duration::from_millis(100),
        };
        assert_eq!(strategy.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(strategy.delay_for_attempt(5), Duration::from_millis(100));
    }

    #[test]
    fn test_exponential_backoff() {
        let strategy = RetryStrategy::ExponentialBackoff {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
        };
        assert_eq!(strategy.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(strategy.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(strategy.delay_for_attempt(2), Duration::from_millis(400));
        assert_eq!(strategy.delay_for_attempt(3), Duration::from_millis(800));
    }

    #[test]
    fn test_exponential_backoff_max_cap() {
        let strategy = RetryStrategy::ExponentialBackoff {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(500),
        };
        // 100 * 2^4 = 1600, capped at 500
        assert_eq!(strategy.delay_for_attempt(4), Duration::from_millis(500));
    }

    #[test]
    fn test_retry_policy_retryable_status() {
        let policy = RetryPolicy::default();
        assert!(policy.is_retryable_status(502));
        assert!(policy.is_retryable_status(503));
        assert!(policy.is_retryable_status(429));
        assert!(!policy.is_retryable_status(200));
        assert!(!policy.is_retryable_status(404));
    }

    #[test]
    fn test_retry_policy_none() {
        let policy = RetryPolicy::none();
        assert_eq!(policy.max_retries, 0);
    }

    #[tokio::test]
    async fn test_retry_async_success_first_try() {
        let policy = RetryPolicy::default();
        let result = vil_server_core::retry::retry_async(&policy, || async {
            Ok::<_, String>("success".to_string())
        })
        .await;
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_retry_async_eventual_success() {
        let policy = RetryPolicy::new(
            3,
            RetryStrategy::Fixed {
                delay: Duration::from_millis(1),
            },
        );
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = vil_server_core::retry::retry_async(&policy, || {
            let c = counter_clone.clone();
            async move {
                let attempt = c.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if attempt < 2 {
                    Err("not yet".to_string())
                } else {
                    Ok("finally".to_string())
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), "finally");
        assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_retry_async_all_fail() {
        let policy = RetryPolicy::new(
            2,
            RetryStrategy::Fixed {
                delay: Duration::from_millis(1),
            },
        );

        let result = vil_server_core::retry::retry_async(&policy, || async {
            Err::<String, _>("always fails".to_string())
        })
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "always fails");
    }
}

// ==================== Feature Flags Tests ====================

#[cfg(test)]
mod feature_flag_tests {
    use vil_server_core::feature_flags::FeatureFlags;

    #[test]
    fn test_define_and_check() {
        let flags = FeatureFlags::new();
        flags.define("new_ui", true, "New user interface");
        flags.define("beta", false, "Beta features");

        assert!(flags.is_enabled("new_ui"));
        assert!(!flags.is_enabled("beta"));
        assert!(!flags.is_enabled("nonexistent"));
    }

    #[test]
    fn test_toggle() {
        let flags = FeatureFlags::new();
        flags.define("feature", true, "test");

        assert!(flags.is_enabled("feature"));
        flags.toggle("feature");
        assert!(!flags.is_enabled("feature"));
        flags.toggle("feature");
        assert!(flags.is_enabled("feature"));
    }

    #[test]
    fn test_rollout_percentage() {
        let flags = FeatureFlags::new();
        flags.define_rollout("experiment", 50, "50% rollout");

        // Deterministic: same entity_id always gets same result
        let result1 = flags.is_enabled_for("experiment", "user_123");
        let result2 = flags.is_enabled_for("experiment", "user_123");
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_rollout_zero_percent() {
        let flags = FeatureFlags::new();
        flags.define_rollout("disabled_rollout", 0, "0% rollout");
        // 0% should never be enabled
        for i in 0..100 {
            assert!(!flags.is_enabled_for("disabled_rollout", &format!("user_{}", i)));
        }
    }

    #[test]
    fn test_rollout_100_percent() {
        let flags = FeatureFlags::new();
        flags.define_rollout("full_rollout", 100, "100% rollout");
        for i in 0..100 {
            assert!(flags.is_enabled_for("full_rollout", &format!("user_{}", i)));
        }
    }

    #[test]
    fn test_load_json() {
        let flags = FeatureFlags::new();
        let json = r#"[
            {"name": "a", "enabled": true, "description": "flag a", "rollout_percentage": 100, "target_ids": []},
            {"name": "b", "enabled": false, "description": "flag b", "rollout_percentage": 0, "target_ids": []}
        ]"#;
        let count = flags.load_json(json).unwrap();
        assert_eq!(count, 2);
        assert!(flags.is_enabled("a"));
        assert!(!flags.is_enabled("b"));
    }

    #[test]
    fn test_list_flags() {
        let flags = FeatureFlags::new();
        flags.define("x", true, "");
        flags.define("y", false, "");
        assert_eq!(flags.count(), 2);
        let list = flags.list();
        assert_eq!(list.len(), 2);
    }
}

// ==================== Timeout Tests ====================

#[cfg(test)]
mod timeout_tests {
    use std::time::Duration;
    use vil_server_core::timeout::TimeoutLayer;

    #[test]
    fn test_timeout_layer_creation() {
        let _layer = TimeoutLayer::new(Duration::from_secs(30));
        let _layer2 = TimeoutLayer::from_secs(60);
    }
}

// ==================== Idempotency Tests ====================

#[cfg(test)]
mod idempotency_tests {
    use axum::http::StatusCode;
    use bytes::Bytes;
    use std::time::Duration;
    use vil_server_core::idempotency::IdempotencyStore;

    #[test]
    fn test_store_put_get() {
        let store = IdempotencyStore::new(Duration::from_secs(60), 100);
        store.put(
            "key1".to_string(),
            StatusCode::OK,
            Bytes::from("response"),
            "application/json".to_string(),
        );

        let result = store.get("key1");
        assert!(result.is_some());
        let (status, body, ct) = result.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, Bytes::from("response"));
        assert_eq!(ct, "application/json");
    }

    #[test]
    fn test_store_miss() {
        let store = IdempotencyStore::new(Duration::from_secs(60), 100);
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn test_store_ttl_expiry() {
        let store = IdempotencyStore::new(Duration::from_millis(10), 100);
        store.put(
            "key1".to_string(),
            StatusCode::OK,
            Bytes::new(),
            "text/plain".to_string(),
        );
        std::thread::sleep(Duration::from_millis(20));
        assert!(store.get("key1").is_none());
    }

    #[test]
    fn test_store_contains() {
        let store = IdempotencyStore::new(Duration::from_secs(60), 100);
        store.put(
            "abc".to_string(),
            StatusCode::OK,
            Bytes::new(),
            "".to_string(),
        );
        assert!(store.contains("abc"));
        assert!(!store.contains("xyz"));
    }

    #[test]
    fn test_store_clear() {
        let store = IdempotencyStore::new(Duration::from_secs(60), 100);
        for i in 0..5 {
            store.put(
                format!("key_{}", i),
                StatusCode::OK,
                Bytes::new(),
                "".to_string(),
            );
        }
        assert_eq!(store.len(), 5);
        store.clear();
        assert!(store.is_empty());
    }
}

// ==================== OTel Tests ====================

#[cfg(test)]
mod otel_tests {
    use vil_server_core::otel::*;

    #[test]
    fn test_trace_id_generate() {
        let t1 = TraceId::generate();
        let t2 = TraceId::generate();
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_trace_id_hex_roundtrip() {
        let t = TraceId::generate();
        let hex = t.to_hex();
        assert_eq!(hex.len(), 32);
        let parsed = TraceId::from_hex(&hex).unwrap();
        assert_eq!(t, parsed);
    }

    #[test]
    fn test_span_id_generate() {
        let s1 = SpanId::generate();
        let s2 = SpanId::generate();
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_span_builder_finish() {
        let span = SpanBuilder::new("test_op", SpanKind::Server, "test-svc")
            .attr("http.method", "GET")
            .attr("http.path", "/api")
            .finish(SpanStatus::Ok);

        assert_eq!(span.name, "test_op");
        assert_eq!(span.service_name, "test-svc");
        assert_eq!(span.attributes.len(), 2);
        assert!(span.duration_ns > 0);
    }

    #[test]
    fn test_span_collector() {
        let collector = SpanCollector::new(100);
        let span = SpanBuilder::new("op", SpanKind::Internal, "svc").finish(SpanStatus::Ok);

        collector.record(span);
        assert_eq!(collector.buffered(), 1);
        assert_eq!(collector.total_collected(), 1);

        let recent = collector.recent(10);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].name, "op");
    }

    #[test]
    fn test_span_collector_drain() {
        let collector = SpanCollector::new(100);
        for i in 0..5 {
            let span = SpanBuilder::new(format!("op_{}", i), SpanKind::Internal, "svc")
                .finish(SpanStatus::Ok);
            collector.record(span);
        }
        assert_eq!(collector.buffered(), 5);
        let drained = collector.drain();
        assert_eq!(drained.len(), 5);
        assert_eq!(collector.buffered(), 0); // drained
        assert_eq!(collector.total_collected(), 5); // total preserved
    }

    #[test]
    fn test_span_collector_ring_buffer() {
        let collector = SpanCollector::new(3);
        for i in 0..5 {
            let span = SpanBuilder::new(format!("op_{}", i), SpanKind::Internal, "svc")
                .finish(SpanStatus::Ok);
            collector.record(span);
        }
        assert_eq!(collector.buffered(), 3); // max size
        assert_eq!(collector.total_collected(), 5);
        let recent = collector.recent(10);
        assert_eq!(recent[0].name, "op_2"); // oldest kept
    }

    #[test]
    fn test_w3c_trace_context_parse() {
        let ctx =
            TraceContext::from_header("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01")
                .unwrap();
        assert!(ctx.sampled);
        assert_eq!(ctx.trace_id.to_hex(), "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.parent_span_id.to_hex(), "b7ad6b7169203331");
    }

    #[test]
    fn test_w3c_trace_context_invalid() {
        assert!(TraceContext::from_header("invalid").is_none());
        assert!(TraceContext::from_header("01-abc-def-00").is_none()); // wrong version
    }

    #[test]
    fn test_w3c_trace_context_roundtrip() {
        let trace_id = TraceId::generate();
        let span_id = SpanId::generate();
        let ctx = TraceContext {
            trace_id,
            parent_span_id: span_id,
            sampled: true,
        };
        let new_span = SpanId::generate();
        let header = ctx.to_header(new_span);
        let parsed = TraceContext::from_header(&header).unwrap();
        assert_eq!(parsed.trace_id, trace_id);
        assert!(parsed.sampled);
    }
}

// ==================== Custom Metrics Tests ====================

#[cfg(test)]
mod custom_metrics_tests {
    use vil_server_core::custom_metrics::CustomMetrics;

    #[test]
    fn test_counter() {
        let m = CustomMetrics::new();
        m.register_counter("requests", "Total requests");
        m.inc("requests");
        m.inc("requests");
        m.inc_by("requests", 3);
        assert_eq!(m.counter_value("requests"), 5);
    }

    #[test]
    fn test_gauge() {
        let m = CustomMetrics::new();
        m.register_gauge("connections", "Active connections");
        m.gauge_set("connections", 42);
        assert_eq!(m.gauge_value("connections"), 42);
        m.gauge_inc("connections");
        assert_eq!(m.gauge_value("connections"), 43);
        m.gauge_dec("connections");
        assert_eq!(m.gauge_value("connections"), 42);
    }

    #[test]
    fn test_histogram() {
        let m = CustomMetrics::new();
        m.register_histogram_default("latency", "Request latency");
        m.observe("latency", 10.0);
        m.observe("latency", 50.0);
        m.observe("latency", 200.0);
        assert_eq!(m.metric_count(), 1);
    }

    #[test]
    fn test_prometheus_export() {
        let m = CustomMetrics::new();
        m.register_counter("test_counter", "A test counter");
        m.inc("test_counter");
        let output = m.to_prometheus();
        assert!(output.contains("test_counter"));
        assert!(output.contains("1"));
    }

    #[test]
    fn test_unregistered_metric() {
        let m = CustomMetrics::new();
        m.inc("nonexistent"); // should not panic
        assert_eq!(m.counter_value("nonexistent"), 0);
    }
}

// ==================== Error Tracker Tests ====================

#[cfg(test)]
mod error_tracker_tests {
    use vil_server_core::error_tracker::ErrorTracker;

    #[test]
    fn test_record_error() {
        let tracker = ErrorTracker::new(100);
        tracker.record("GET", "/api/users", 500, "Internal error", Some("req-123"));
        assert_eq!(tracker.error_count(), 1);
        assert_eq!(tracker.pattern_count(), 1);
    }

    #[test]
    fn test_error_patterns() {
        let tracker = ErrorTracker::new(100);
        // Same pattern 3 times
        for _ in 0..3 {
            tracker.record("GET", "/api/users", 500, "DB connection failed", None);
        }
        // Different pattern
        tracker.record("POST", "/api/orders", 400, "Invalid input", None);

        assert_eq!(tracker.error_count(), 4);
        assert_eq!(tracker.pattern_count(), 2);

        let top = tracker.top_patterns(10);
        assert_eq!(top[0].count, 3); // Most frequent first
    }

    #[test]
    fn test_recent_errors() {
        let tracker = ErrorTracker::new(5);
        for i in 0..10 {
            tracker.record("GET", &format!("/path/{}", i), 500, "error", None);
        }
        let recent = tracker.recent(5);
        assert_eq!(recent.len(), 5);
        // Should be the last 5
        assert!(recent[0].path.contains("5")); // ring buffer kept last 5
    }

    #[test]
    fn test_clear() {
        let tracker = ErrorTracker::new(100);
        tracker.record("GET", "/", 500, "err", None);
        tracker.clear();
        assert_eq!(tracker.error_count(), 0);
        assert_eq!(tracker.pattern_count(), 0);
    }
}

// ==================== Alerting Tests ====================

#[cfg(test)]
mod alerting_tests {
    use std::collections::HashMap;
    use std::time::Duration;
    use vil_server_core::alerting::*;

    #[test]
    fn test_alert_condition_gt() {
        let cond = AlertCondition::GreaterThan(80.0);
        assert!(cond.evaluate(90.0));
        assert!(!cond.evaluate(70.0));
        assert!(!cond.evaluate(80.0));
    }

    #[test]
    fn test_alert_condition_lt() {
        let cond = AlertCondition::LessThan(10.0);
        assert!(cond.evaluate(5.0));
        assert!(!cond.evaluate(15.0));
    }

    #[test]
    fn test_alert_engine_ok_state() {
        let mut engine = AlertEngine::new();
        engine.add_rule(AlertRule {
            name: "high_cpu".to_string(),
            description: "CPU too high".to_string(),
            severity: AlertSeverity::Warning,
            metric: "cpu_usage".to_string(),
            condition: AlertCondition::GreaterThan(80.0),
            for_duration: Duration::from_secs(0),
        });

        let mut metrics = HashMap::new();
        metrics.insert("cpu_usage".to_string(), 50.0);

        let statuses = engine.evaluate(&metrics);
        assert_eq!(statuses[0].state, AlertState::Ok);
    }

    #[test]
    fn test_alert_engine_firing() {
        let mut engine = AlertEngine::new();
        engine.add_rule(AlertRule {
            name: "high_errors".to_string(),
            description: "Error rate high".to_string(),
            severity: AlertSeverity::Critical,
            metric: "error_rate".to_string(),
            condition: AlertCondition::GreaterThan(5.0),
            for_duration: Duration::from_secs(0), // Immediate
        });

        let mut metrics = HashMap::new();
        metrics.insert("error_rate".to_string(), 10.0);

        // First eval: Pending
        let statuses = engine.evaluate(&metrics);
        assert_eq!(statuses[0].state, AlertState::Pending);

        // Second eval: Firing (for_duration=0, already pending)
        let statuses = engine.evaluate(&metrics);
        assert_eq!(statuses[0].state, AlertState::Firing);
    }

    #[test]
    fn test_alert_engine_resolved() {
        let mut engine = AlertEngine::new();
        engine.add_rule(AlertRule {
            name: "test".to_string(),
            description: "test".to_string(),
            severity: AlertSeverity::Info,
            metric: "val".to_string(),
            condition: AlertCondition::GreaterThan(50.0),
            for_duration: Duration::from_secs(0),
        });

        let mut metrics = HashMap::new();
        metrics.insert("val".to_string(), 100.0);

        // Trigger
        engine.evaluate(&metrics);
        engine.evaluate(&metrics); // Firing

        // Resolve
        metrics.insert("val".to_string(), 30.0);
        let statuses = engine.evaluate(&metrics);
        assert_eq!(statuses[0].state, AlertState::Resolved);
    }
}

// ==================== Content Negotiation Tests ====================

#[cfg(test)]
mod content_negotiation_tests {
    use vil_server_core::content_negotiation::ContentType;

    #[test]
    fn test_content_type_mime() {
        assert_eq!(ContentType::Json.mime(), "application/json");
        assert_eq!(ContentType::Plain.mime(), "text/plain");
        assert_eq!(ContentType::Html.mime(), "text/html");
    }
}

// ==================== API Versioning Tests ====================

#[cfg(test)]
mod api_versioning_tests {
    use vil_server_core::api_versioning::ApiVersion;

    #[test]
    fn test_api_version_display() {
        let v = ApiVersion::v1();
        assert_eq!(v.to_string(), "v1.0");
        assert!(v.is_v1());
        assert!(!v.is_v2());
    }

    #[test]
    fn test_api_version_v2() {
        let v = ApiVersion::v2();
        assert_eq!(v.to_string(), "v2.0");
        assert!(v.is_v2());
    }
}

// ==================== Rolling Restart Tests ====================

#[cfg(test)]
mod rolling_restart_tests {
    use std::time::Duration;
    use vil_server_core::rolling_restart::*;

    #[test]
    fn test_initial_state() {
        let coord = RestartCoordinator::new(Duration::from_secs(30));
        assert_eq!(coord.phase(), RestartPhase::Running);
        assert!(coord.is_accepting());
        assert_eq!(coord.in_flight(), 0);
    }

    #[test]
    fn test_drain_phase() {
        let coord = RestartCoordinator::new(Duration::from_secs(30));
        coord.request_enter();
        coord.request_enter();
        assert_eq!(coord.in_flight(), 2);

        coord.start_drain();
        assert_eq!(coord.phase(), RestartPhase::Draining);
        assert!(!coord.is_accepting());

        coord.request_exit();
        assert_eq!(coord.in_flight(), 1);
        assert_eq!(coord.phase(), RestartPhase::Draining);

        coord.request_exit();
        assert_eq!(coord.in_flight(), 0);
        assert_eq!(coord.phase(), RestartPhase::ShuttingDown);
    }

    #[test]
    fn test_status() {
        let coord = RestartCoordinator::default();
        let status = coord.status();
        assert_eq!(status.phase, RestartPhase::Running);
        assert!(status.accepting);
        assert_eq!(status.drain_timeout_secs, 30);
    }
}

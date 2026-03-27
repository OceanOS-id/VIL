// Integration tests for vil_server_macros
//
// Tests cover:
// 1. VilSseEvent derive — to_sse_event(), default topic, custom topic
// 2. vil_handler attribute — compilation test with Result and plain returns

use serde::Serialize;
use vil_server_macros::VilSseEvent;

// ---------------------------------------------------------------------------
// 1. VilSseEvent derive tests
// ---------------------------------------------------------------------------

#[derive(Serialize, VilSseEvent)]
struct OrderCreated {
    order_id: u64,
    total: f64,
}

#[derive(Serialize, VilSseEvent)]
#[sse_event(topic = "custom_topic")]
struct CustomEvent {
    payload: String,
}

#[derive(Serialize, VilSseEvent)]
struct EmptyEvent {}

#[test]
fn sse_event_to_sse_event_returns_ok() {
    let event = OrderCreated {
        order_id: 42,
        total: 99.95,
    };
    let result = event.to_sse_event();
    assert!(result.is_ok(), "to_sse_event() should return Ok(Event)");
}

#[test]
fn sse_event_default_topic_is_lowercase_struct_name() {
    // The default topic for `OrderCreated` should be "ordercreated".
    // We verify by inspecting the Debug output of the Event, which includes
    // the event name.
    let event = OrderCreated {
        order_id: 1,
        total: 10.0,
    };
    let sse = event.to_sse_event().unwrap();
    let debug_str = format!("{:?}", sse);
    assert!(
        debug_str.contains("ordercreated"),
        "Default topic should be lowercase struct name 'ordercreated', got: {}",
        debug_str
    );
}

#[test]
fn sse_event_custom_topic_attribute() {
    let event = CustomEvent {
        payload: "hello".into(),
    };
    let sse = event.to_sse_event().unwrap();
    let debug_str = format!("{:?}", sse);
    assert!(
        debug_str.contains("custom_topic"),
        "Custom topic should be 'custom_topic', got: {}",
        debug_str
    );
}

#[test]
fn sse_event_data_contains_json() {
    let event = OrderCreated {
        order_id: 7,
        total: 3.14,
    };
    let sse = event.to_sse_event().unwrap();
    let debug_str = format!("{:?}", sse);
    // The JSON serialization should contain the field values
    assert!(
        debug_str.contains("order_id"),
        "Event data should contain JSON field 'order_id', got: {}",
        debug_str
    );
}

#[test]
fn sse_event_empty_struct_works() {
    let event = EmptyEvent {};
    let result = event.to_sse_event();
    assert!(result.is_ok(), "Empty struct should produce a valid SSE event");
}

#[test]
fn sse_event_broadcast_does_not_panic() {
    // Verify that broadcast() works without panicking when given a hub.
    let hub = vil_server_core::streaming::SseHub::new(16);
    let event = OrderCreated {
        order_id: 99,
        total: 42.0,
    };
    // Should not panic even with no subscribers
    event.broadcast(&hub);
}

// ---------------------------------------------------------------------------
// 2. vil_handler attribute — compilation tests
// ---------------------------------------------------------------------------
// These tests verify that the macro-generated code compiles correctly.
// Since vil_handler produces code that references vil_server_core types
// (RequestId, VilResponse, VilError) and axum::response::Response,
// we test that the generated functions have the expected signatures.

mod handler_compilation {
    use vil_server_macros::vil_handler;
    use vil_server_core::VilError;

    /// A handler returning Result — the macro should wrap it with
    /// Ok path -> VilResponse::ok and Err path -> VilError mapping.
    #[vil_handler]
    async fn result_handler(name: String) -> Result<String, VilError> {
        Ok(format!("hello {}", name))
    }

    /// A handler returning a plain value — the macro should wrap it
    /// with VilResponse::ok.
    #[vil_handler]
    async fn plain_handler() -> &'static str {
        "ok"
    }

    /// A handler with multiple parameters.
    #[vil_handler]
    async fn multi_param_handler(a: u32, b: String) -> Result<String, VilError> {
        Ok(format!("{} {}", a, b))
    }

    #[test]
    fn result_handler_compiles_and_is_async() {
        // The wrapper function `result_handler` should exist and accept
        // RequestId as first param. We just verify it is callable.
        // Actually calling it requires a tokio runtime + real RequestId,
        // so we verify the inner function directly.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = __vil_inner_result_handler("world".to_string()).await;
            assert_eq!(result.unwrap(), "hello world");
        });
    }

    #[test]
    fn plain_handler_compiles() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = __vil_inner_plain_handler().await;
            assert_eq!(result, "ok");
        });
    }

    #[test]
    fn multi_param_handler_inner_works() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = __vil_inner_multi_param_handler(42, "test".to_string()).await;
            assert_eq!(result.unwrap(), "42 test");
        });
    }

    #[test]
    fn wrapper_function_has_request_id_param() {
        // Verify the wrapper accepts RequestId as first argument and returns Response.
        // We call the generated wrapper with a real RequestId.
        use axum::response::Response;
        use vil_server_core::RequestId;

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let rid = RequestId("test-rid-123".to_string());
            let response: Response = result_handler(rid, "vil".to_string()).await;
            assert_eq!(response.status(), axum::http::StatusCode::OK);
        });
    }

    #[test]
    fn wrapper_plain_handler_returns_response() {
        use axum::response::Response;
        use vil_server_core::RequestId;

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let rid = RequestId("rid-456".to_string());
            let response: Response = plain_handler(rid).await;
            assert_eq!(response.status(), axum::http::StatusCode::OK);
        });
    }
}

// ---------------------------------------------------------------------------
// 3. vil_endpoint attribute — compilation tests
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 3. vil_service_state attribute — compilation tests
// ---------------------------------------------------------------------------

mod service_state_compilation {
    use vil_server_macros::vil_service_state;

    #[vil_service_state]
    #[allow(dead_code)]
    struct DefaultState {
        value: u64,
    }

    #[vil_service_state(storage = PrivateHeap)]
    #[allow(dead_code)]
    struct ExplicitHeapState {
        count: u32,
    }

    #[vil_service_state(storage = SharedShm)]
    #[allow(dead_code)]
    struct SharedState {
        data: String,
    }

    #[test]
    fn vil_service_state_default_storage() {
        assert!(DefaultState::VIL_SERVICE_STATE);
        assert_eq!(DefaultState::VIL_STATE_STORAGE, "PrivateHeap");
    }

    #[test]
    fn vil_service_state_explicit_storage() {
        assert!(ExplicitHeapState::VIL_SERVICE_STATE);
        assert_eq!(ExplicitHeapState::VIL_STATE_STORAGE, "PrivateHeap");

        assert!(SharedState::VIL_SERVICE_STATE);
        assert_eq!(SharedState::VIL_STATE_STORAGE, "SharedShm");
    }
}

// ---------------------------------------------------------------------------
// 4. vil_service attribute — compilation tests
// ---------------------------------------------------------------------------

mod service_compilation {
    use vil_server_macros::vil_service;

    #[vil_service(name = "orders", prefix = "/api")]
    mod orders {
        // Intentionally empty — the macro should still inject items.
    }

    #[vil_service(name = "payments", prefix = "/pay", requires = ["auth:Trigger", "orders:Data"])]
    mod payments {}

    #[vil_service(name = "inventory")]
    mod inventory {}

    #[test]
    fn vil_service_generates_factory() {
        // Verify that constants are generated correctly
        assert_eq!(orders::SERVICE_NAME, "orders");
        assert_eq!(orders::SERVICE_PREFIX, "/api");
        assert!(orders::MESH_REQUIRES.is_empty());

        // Verify the service() factory returns a ServiceProcess with correct name/prefix
        let svc = orders::service();
        assert_eq!(svc.name(), "orders");
        assert_eq!(svc.prefix_path(), "/api");
    }

    #[test]
    fn vil_service_with_requires() {
        assert_eq!(payments::SERVICE_NAME, "payments");
        assert_eq!(payments::SERVICE_PREFIX, "/pay");
        assert_eq!(payments::MESH_REQUIRES, &["auth:Trigger", "orders:Data"]);

        let svc = payments::service();
        assert_eq!(svc.name(), "payments");
        assert_eq!(svc.prefix_path(), "/pay");
    }

    #[test]
    fn vil_service_default_prefix() {
        // When prefix is omitted, it defaults to /api/{name}
        assert_eq!(inventory::SERVICE_NAME, "inventory");
        assert_eq!(inventory::SERVICE_PREFIX, "/api/inventory");

        let svc = inventory::service();
        assert_eq!(svc.prefix_path(), "/api/inventory");
    }
}

// ---------------------------------------------------------------------------
// 5. vil_endpoint attribute — compilation tests
// ---------------------------------------------------------------------------

mod endpoint_compilation {
    use vil_server_macros::vil_endpoint;

    /// Default exec class (AsyncTask) — simple async endpoint.
    #[vil_endpoint]
    async fn simple_endpoint() -> &'static str {
        "hello from vx"
    }

    /// Endpoint with parameters — default AsyncTask.
    #[vil_endpoint]
    async fn endpoint_with_params(name: String, count: u32) -> String {
        format!("{}: {}", name, count)
    }

    /// Endpoint with explicit AsyncTask exec class.
    #[vil_endpoint(exec = AsyncTask)]
    async fn async_endpoint() -> &'static str {
        "async task"
    }

    /// Endpoint with BlockingTask exec class — body runs in spawn_blocking.
    #[vil_endpoint(exec = BlockingTask)]
    async fn blocking_endpoint(input: u64) -> u64 {
        // Simulates CPU-bound work
        input * 2
    }

    /// Endpoint with DedicatedThread exec class.
    #[vil_endpoint(exec = DedicatedThread)]
    async fn dedicated_endpoint() -> String {
        "dedicated".to_string()
    }

    #[test]
    fn simple_endpoint_compiles_and_runs() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = simple_endpoint().await;
            assert_eq!(result, "hello from vx");
        });
    }

    #[test]
    fn endpoint_with_params_compiles_and_runs() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = endpoint_with_params("test".to_string(), 42).await;
            assert_eq!(result, "test: 42");
        });
    }

    #[test]
    fn async_endpoint_compiles_and_runs() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = async_endpoint().await;
            assert_eq!(result, "async task");
        });
    }

    #[test]
    fn blocking_endpoint_compiles_and_runs() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = blocking_endpoint(21).await;
            assert_eq!(result, 42);
        });
    }

    #[test]
    fn dedicated_endpoint_compiles_and_runs() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = dedicated_endpoint().await;
            assert_eq!(result, "dedicated");
        });
    }
}

// ---------------------------------------------------------------------------
// 6. vil_endpoint — auto body extraction tests
// ---------------------------------------------------------------------------

mod endpoint_auto_extraction {
    use axum::extract::Json;
    use serde::{Deserialize, Serialize};
    use vil_server_macros::vil_endpoint;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct CreateOrder {
        item: String,
        quantity: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[allow(dead_code)]
    struct Order {
        id: u64,
        item: String,
    }

    // Test 1: Auto body extraction — `body: CreateOrder` should be rewritten
    // to `Json(body): Json<CreateOrder>`.
    // We verify by calling the function with a Json wrapper.
    #[vil_endpoint]
    async fn create_order_auto(body: CreateOrder) -> String {
        format!("created: {}", body.item)
    }

    #[test]
    fn endpoint_auto_body_extraction() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let order = CreateOrder {
                item: "widget".to_string(),
                quantity: 5,
            };
            // The macro rewrites `body: CreateOrder` to `Json(body): Json<CreateOrder>`,
            // so we must call with Json wrapper.
            let result = create_order_auto(Json(order)).await;
            assert_eq!(result, "created: widget");
        });
    }

    // Test 2: Mixed params — body gets wrapped, Path extractor stays as-is.
    #[vil_endpoint]
    async fn update_order(
        path_param: axum::extract::Path<u64>,
        body: CreateOrder,
    ) -> String {
        let id = path_param.0;
        format!("update order {} with {}", id, body.item)
    }

    #[test]
    fn endpoint_mixed_params() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = update_order(
                axum::extract::Path(42),
                Json(CreateOrder {
                    item: "gadget".to_string(),
                    quantity: 3,
                }),
            )
            .await;
            assert_eq!(result, "update order 42 with gadget");
        });
    }

    // Test 3: Known extractor Json is not double-wrapped.
    #[vil_endpoint]
    async fn explicit_json(Json(body): Json<CreateOrder>) -> String {
        format!("explicit: {}", body.item)
    }

    #[test]
    fn endpoint_known_extractor_not_wrapped() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let order = CreateOrder {
                item: "thing".to_string(),
                quantity: 1,
            };
            let result = explicit_json(Json(order)).await;
            assert_eq!(result, "explicit: thing");
        });
    }

    // Test 4: String parameter (known type) is not wrapped.
    #[vil_endpoint]
    async fn string_param(name: String) -> String {
        format!("hello {}", name)
    }

    #[test]
    fn endpoint_service_ctx_not_wrapped() {
        // String is a known type — it should NOT be wrapped with Json.
        // We verify by calling with a plain String (not Json<String>).
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = string_param("world".to_string()).await;
            assert_eq!(result, "hello world");
        });
    }
}

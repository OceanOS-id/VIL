// =============================================================================
// Integration tests for VilModel and VilError derive macros
// =============================================================================

use serde::{Deserialize, Serialize};
use vil_macros::{VilError, VilModel};
use vil_server_core::model::VilModel;

// =============================================================================
// Test types
// =============================================================================

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, VilModel)]
struct Task {
    id: u64,
    title: String,
    done: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, VilModel)]
struct EmptyModel {}

#[derive(Debug, VilError)]
enum TaskError {
    #[vil_error(status = 404)]
    NotFound { id: u64 },

    #[vil_error(status = 400)]
    InvalidTitle,

    #[vil_error(status = 500)]
    DatabaseError(String),
}

#[derive(Debug, VilError)]
enum EnrichedError {
    #[vil_error(status = 404, code = "TASK_NOT_FOUND")]
    NotFound { id: u64 },

    #[vil_error(status = 400, code = "INVALID_TITLE", retry = false)]
    InvalidTitle,

    #[vil_error(status = 503, code = "DB_UNAVAILABLE", retry = true)]
    DatabaseDown(String),
}

// =============================================================================
// VilModel derive tests
// =============================================================================

#[test]
fn vil_model_from_shm_bytes_roundtrip() {
    let original = Task {
        id: 42,
        title: "Write tests".to_string(),
        done: false,
    };

    // Serialize to JSON bytes via vil_json
    let json_bytes = vil_json::to_vec(&original).expect("serialize");

    // Deserialize through the derived from_shm_bytes
    let restored = Task::from_shm_bytes(&json_bytes).expect("from_shm_bytes");
    assert_eq!(original, restored);
}

#[test]
fn vil_model_to_json_bytes_produces_valid_json() {
    let task = Task {
        id: 7,
        title: "Deploy".to_string(),
        done: true,
    };

    let bytes = task.to_json_bytes().expect("to_json_bytes");
    assert!(!bytes.is_empty());

    // The bytes must be valid JSON that round-trips back
    let parsed: serde_json::Value =
        serde_json::from_slice(&bytes).expect("should be valid JSON");

    assert_eq!(parsed["id"], 7);
    assert_eq!(parsed["title"], "Deploy");
    assert_eq!(parsed["done"], true);
}

#[test]
fn vil_model_to_json_bytes_then_from_shm_bytes() {
    let original = Task {
        id: 100,
        title: "Full roundtrip".to_string(),
        done: true,
    };

    let bytes = original.to_json_bytes().expect("to_json_bytes");
    let restored = Task::from_shm_bytes(&bytes).expect("from_shm_bytes");
    assert_eq!(original, restored);
}

#[test]
fn vil_model_from_shm_bytes_invalid_bytes() {
    let garbage = b"this is not valid json";
    let result = Task::from_shm_bytes(garbage);
    assert!(result.is_err(), "should fail on invalid bytes");

    let err = result.unwrap_err();
    // VilError::bad_request produces status 400
    assert_eq!(err.status.as_u16(), 400);
}

#[test]
fn vil_model_from_shm_bytes_wrong_schema() {
    // Valid JSON but wrong shape — missing required fields
    let wrong = br#"{"unexpected": true}"#;
    let result = Task::from_shm_bytes(wrong);
    assert!(result.is_err(), "should fail on schema mismatch");
}

#[test]
fn vil_model_empty_struct_roundtrip() {
    let original = EmptyModel {};
    let bytes = original.to_json_bytes().expect("to_json_bytes");
    let restored = EmptyModel::from_shm_bytes(&bytes).expect("from_shm_bytes");
    assert_eq!(original, restored);
}

// =============================================================================
// VilError derive tests
// =============================================================================

#[test]
fn vil_error_display_unit_variant() {
    let err = TaskError::InvalidTitle;
    let msg = format!("{}", err);
    assert_eq!(msg, "InvalidTitle");
}

#[test]
fn vil_error_display_named_fields_variant() {
    let err = TaskError::NotFound { id: 99 };
    let msg = format!("{}", err);
    assert_eq!(msg, "NotFound: id=99");
}

#[test]
fn vil_error_display_tuple_variant() {
    let err = TaskError::DatabaseError("connection refused".to_string());
    let msg = format!("{}", err);
    assert_eq!(msg, "DatabaseError: connection refused");
}

#[test]
fn vil_error_from_maps_not_found_to_404() {
    let err = TaskError::NotFound { id: 1 };
    let vil_err: vil_server_core::VilError = err.into();
    assert_eq!(vil_err.status.as_u16(), 404);
}

#[test]
fn vil_error_from_maps_invalid_title_to_400() {
    let err = TaskError::InvalidTitle;
    let vil_err: vil_server_core::VilError = err.into();
    assert_eq!(vil_err.status.as_u16(), 400);
}

#[test]
fn vil_error_from_maps_database_error_to_500() {
    let err = TaskError::DatabaseError("timeout".to_string());
    let vil_err: vil_server_core::VilError = err.into();
    assert_eq!(vil_err.status.as_u16(), 500);
}

#[test]
fn vil_error_detail_contains_display_message() {
    let err = TaskError::NotFound { id: 42 };
    let vil_err: vil_server_core::VilError = err.into();
    assert!(
        vil_err.detail.contains("NotFound"),
        "detail should contain variant name, got: {}",
        vil_err.detail
    );
    assert!(
        vil_err.detail.contains("42"),
        "detail should contain field value, got: {}",
        vil_err.detail
    );
}

#[test]
fn vil_error_implements_std_error() {
    let err = TaskError::InvalidTitle;
    // Confirm it implements std::error::Error by using it as a trait object
    let _dyn_err: &dyn std::error::Error = &err;
}

// =============================================================================
// Enriched VilError derive tests (code + retry attributes)
// =============================================================================

#[test]
fn vil_error_with_code_includes_code_in_detail() {
    let err = EnrichedError::NotFound { id: 42 };
    let vil_err: vil_server_core::VilError = err.into();
    assert_eq!(vil_err.status.as_u16(), 404);
    assert!(
        vil_err.detail.contains("[TASK_NOT_FOUND]"),
        "detail should contain error code, got: {}",
        vil_err.detail
    );
    assert!(
        vil_err.detail.contains("42"),
        "detail should contain field value, got: {}",
        vil_err.detail
    );
}

#[test]
fn vil_error_with_code_unit_variant() {
    let err = EnrichedError::InvalidTitle;
    let vil_err: vil_server_core::VilError = err.into();
    assert_eq!(vil_err.status.as_u16(), 400);
    assert!(
        vil_err.detail.contains("[INVALID_TITLE]"),
        "detail should contain error code, got: {}",
        vil_err.detail
    );
}

#[test]
fn vil_error_with_code_tuple_variant() {
    let err = EnrichedError::DatabaseDown("connection reset".to_string());
    let vil_err: vil_server_core::VilError = err.into();
    assert_eq!(vil_err.status.as_u16(), 503);
    assert!(
        vil_err.detail.contains("[DB_UNAVAILABLE]"),
        "detail should contain error code, got: {}",
        vil_err.detail
    );
    assert!(
        vil_err.detail.contains("connection reset"),
        "detail should contain original message, got: {}",
        vil_err.detail
    );
}

#[test]
fn vil_error_with_retry_attribute() {
    // retry = false for InvalidTitle
    assert!(!EnrichedError::INVALID_TITLE_RETRY);
    // retry = true for DatabaseDown
    assert!(EnrichedError::DATABASE_DOWN_RETRY);
}

#[test]
fn vil_error_code_constants_generated() {
    assert_eq!(EnrichedError::NOT_FOUND_CODE, "TASK_NOT_FOUND");
    assert_eq!(EnrichedError::INVALID_TITLE_CODE, "INVALID_TITLE");
    assert_eq!(EnrichedError::DATABASE_DOWN_CODE, "DB_UNAVAILABLE");
}

#[test]
fn vil_error_retry_defaults_to_false_without_attribute() {
    // TaskError variants have no retry attribute, so defaults to false
    assert!(!TaskError::NOT_FOUND_RETRY);
    assert!(!TaskError::INVALID_TITLE_RETRY);
    assert!(!TaskError::DATABASE_ERROR_RETRY);
}

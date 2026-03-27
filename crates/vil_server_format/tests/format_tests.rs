// =============================================================================
// V9 FormatResponse — Unit Tests
// =============================================================================

#[test]
fn test_negotiate_json_default() {
    use vil_server_format::negotiator;
    let fmt = negotiator::negotiate(None);
    assert_eq!(fmt, negotiator::ResponseFormat::Json);
}

#[test]
fn test_negotiate_json_explicit() {
    use vil_server_format::negotiator;
    let fmt = negotiator::negotiate(Some("application/json"));
    assert_eq!(fmt, negotiator::ResponseFormat::Json);
}

#[test]
fn test_negotiate_wildcard() {
    use vil_server_format::negotiator;
    let fmt = negotiator::negotiate(Some("*/*"));
    assert_eq!(fmt, negotiator::ResponseFormat::Json); // default
}

#[test]
fn test_is_supported_json() {
    use vil_server_format::negotiator;
    assert!(negotiator::is_supported("application/json"));
    assert!(negotiator::is_supported("*/*"));
}

#[test]
fn test_is_supported_unknown() {
    use vil_server_format::negotiator;
    assert!(!negotiator::is_supported("application/xml"));
    assert!(!negotiator::is_supported("text/csv"));
}

#[cfg(feature = "protobuf")]
#[test]
fn test_negotiate_protobuf() {
    use vil_server_format::negotiator;
    let fmt = negotiator::negotiate(Some("application/protobuf"));
    assert_eq!(fmt, negotiator::ResponseFormat::Protobuf);
}

#[test]
fn test_response_format_content_type() {
    use vil_server_format::ResponseFormat;
    assert_eq!(ResponseFormat::Json.content_type(), "application/json");
}

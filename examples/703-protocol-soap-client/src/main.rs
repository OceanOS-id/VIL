// =============================================================================
// example-703-protocol-soap-client — SOAP client calling a web service
// =============================================================================
//
// Demonstrates:
//   - SoapClient::new() with a public SOAP endpoint
//   - call_action() to invoke a SOAP operation
//   - ParsedResponse.body_xml and .is_fault inspection
//   - db_log! auto-emitted (op_type=4 CALL) on every call
//   - StdoutDrain::resolved() output
//
// Uses the free public NumberConversion SOAP service for demonstration.
// If the service is unreachable, the example exits gracefully.
//
// To test with a local mock:
//   docker run -p 8080:8080 soapui/soapui-mock-service:latest
// =============================================================================

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};
use vil_soap::{SoapClient, SoapConfig};

/// Public demo SOAP endpoint — NumberConversion service.
const WSDL_URL: &str =
    "https://www.dataaccess.com/webservicesserver/NumberConversion.wso?wsdl";
const ENDPOINT: &str =
    "https://www.dataaccess.com/webservicesserver/NumberConversion.wso";
const NS: &str = "https://www.dataaccess.com/webservicesserver/";

#[tokio::main]
async fn main() {
    // ── Init vil_log with resolved drain ──
    let config = LogConfig {
        ring_slots:        4096,
        level:             LogLevel::Info,
        batch_size:        64,
        flush_interval_ms: 50,
        threads:           None,
        dict_path: None,
        fallback_path: None,
        drain_failure_threshold: 3,
    };
    let _task = init_logging(config, StdoutDrain::new(StdoutFormat::Resolved));

    println!();
    println!("  example-703-protocol-soap-client");
    println!("  SOAP client with db_log! auto-emit (op_type=4 CALL)");
    println!();
    println!("  WSDL:     {}", WSDL_URL);
    println!("  Endpoint: {}", ENDPOINT);
    println!();
    println!("  NOTE: Calls public NumberConversion SOAP service.");
    println!("  Network access required. If offline, example exits gracefully.");
    println!();

    let soap_cfg = SoapConfig::new(WSDL_URL, ENDPOINT).with_timeout_ms(10_000);

    let client = match SoapClient::new(soap_cfg) {
        Ok(c)  => c,
        Err(e) => {
            println!("  [SKIP] Cannot build SOAP client: {:?}", e);
            return;
        }
    };

    // ── Call NumberToWords(42) ──
    println!("  Calling NumberToWords(42)...");
    let body_xml = "<NumberToWords>\
        <ubiNum>42</ubiNum>\
    </NumberToWords>";

    match client.call_action("NumberToWords", NS, body_xml).await {
        Ok(resp) => {
            println!("  Action:    NumberToWords");
            println!("  Fault:     {}", resp.is_fault);
            println!("  Body XML:  {}", resp.body_xml.trim());
        }
        Err(e) => {
            println!("  [SKIP] SOAP call failed (network/service unreachable): {:?}", e);
            println!();
            println!("  In production, a successful call emits:");
            println!("    db_log! {{ op_type=4(CALL), db_hash=<endpoint>, table_hash=<action> }}");
            // Allow drain to flush and exit
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            return;
        }
    }

    // ── Call NumberToDollars(1000) ──
    println!();
    println!("  Calling NumberToDollars(1000)...");
    let body_xml2 = "<NumberToDollars>\
        <dNum>1000</dNum>\
    </NumberToDollars>";

    match client.call_action("NumberToDollars", NS, body_xml2).await {
        Ok(resp) => {
            println!("  Action:    NumberToDollars");
            println!("  Fault:     {}", resp.is_fault);
            println!("  Body XML:  {}", resp.body_xml.trim());
        }
        Err(e) => println!("  [SKIP] {:?}", e),
    }

    // Allow drain to flush
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!();
    println!("  Done. db_log! entries (op_type=4 CALL) emitted above.");
    println!();
}

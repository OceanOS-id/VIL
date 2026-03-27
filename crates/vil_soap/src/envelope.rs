// =============================================================================
// vil_soap::envelope — SOAP XML envelope builder and parser
// =============================================================================
//
// Builds SOAP 1.1 envelopes and parses SOAP responses using quick-xml.
// No heap allocations on the hot path beyond the required XML string buffer.
// =============================================================================

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use quick_xml::Reader;
use std::io::Cursor;

use crate::error::SoapFault;
use vil_log::dict::register_str;

/// Namespace for SOAP 1.1 envelope.
const SOAP_NS: &str = "http://schemas.xmlsoap.org/soap/envelope/";

/// Build a SOAP 1.1 envelope wrapping the given body XML fragment.
///
/// # Parameters
/// - `action`   — SOAP action name, used as the wrapper element tag.
/// - `ns`       — Target namespace for the action element.
/// - `body_xml` — Inner XML to place inside the SOAP Body.
pub fn build_envelope(action: &str, ns: &str, body_xml: &str) -> Result<String, SoapFault> {
    let action_hash = register_str(action);
    let mut buf = Vec::with_capacity(512);
    let mut w = Writer::new(Cursor::new(&mut buf));

    // <?xml version="1.0" encoding="UTF-8"?>
    w.write_event(Event::PI(quick_xml::events::BytesPI::new(
        "xml version=\"1.0\" encoding=\"UTF-8\"",
    )))
    .map_err(|_| SoapFault::EnvelopeBuildFailed { action_hash })?;

    // <soapenv:Envelope>
    let mut env_start = BytesStart::new("soapenv:Envelope");
    env_start.push_attribute(("xmlns:soapenv", SOAP_NS));
    env_start.push_attribute(("xmlns:tns", ns));
    w.write_event(Event::Start(env_start))
        .map_err(|_| SoapFault::EnvelopeBuildFailed { action_hash })?;

    // <soapenv:Header/>
    w.write_event(Event::Empty(BytesStart::new("soapenv:Header")))
        .map_err(|_| SoapFault::EnvelopeBuildFailed { action_hash })?;

    // <soapenv:Body>
    w.write_event(Event::Start(BytesStart::new("soapenv:Body")))
        .map_err(|_| SoapFault::EnvelopeBuildFailed { action_hash })?;

    // Raw inner body XML
    w.write_event(Event::Text(BytesText::from_escaped(body_xml)))
        .map_err(|_| SoapFault::EnvelopeBuildFailed { action_hash })?;

    // </soapenv:Body>
    w.write_event(Event::End(BytesEnd::new("soapenv:Body")))
        .map_err(|_| SoapFault::EnvelopeBuildFailed { action_hash })?;

    // </soapenv:Envelope>
    w.write_event(Event::End(BytesEnd::new("soapenv:Envelope")))
        .map_err(|_| SoapFault::EnvelopeBuildFailed { action_hash })?;

    String::from_utf8(buf).map_err(|_| SoapFault::EnvelopeBuildFailed { action_hash })
}

/// Parsed result of a SOAP response envelope.
pub struct ParsedResponse {
    /// Raw XML content of the SOAP Body element.
    pub body_xml: String,
    /// Whether a SOAP Fault was found.
    pub is_fault: bool,
    /// FxHash of the faultcode string (0 if no fault).
    pub faultcode_hash: u32,
    /// FxHash of the faultstring (0 if no fault).
    pub faultstring_hash: u32,
}

/// Parse a raw SOAP XML response string.
///
/// Extracts the Body content and detects SOAP Fault elements.
pub fn parse_envelope(xml: &str, action: &str) -> Result<ParsedResponse, SoapFault> {
    let action_hash = register_str(action);
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut in_body = false;
    let mut in_fault = false;
    let mut body_buf = String::new();
    let mut faultcode = String::new();
    let mut faultstring = String::new();
    let mut current_tag = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_tag = name.clone();
                if name.ends_with("Body") {
                    in_body = true;
                } else if name.ends_with("Fault") && in_body {
                    in_fault = true;
                } else if in_body {
                    body_buf.push_str(&format!("<{}>", name));
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name.ends_with("Body") {
                    in_body = false;
                } else if name.ends_with("Fault") {
                    in_fault = false;
                } else if in_body {
                    body_buf.push_str(&format!("</{}>", name));
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_body {
                    let text = e.unescape().unwrap_or_default();
                    if in_fault {
                        if current_tag.ends_with("faultcode") {
                            faultcode = text.to_string();
                        } else if current_tag.ends_with("faultstring") {
                            faultstring = text.to_string();
                        }
                    } else {
                        body_buf.push_str(&text);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => {
                return Err(SoapFault::EnvelopeParseFailed {
                    action_hash,
                    reason_code: 1,
                });
            }
            _ => {}
        }
        buf.clear();
    }

    let is_fault = !faultcode.is_empty();
    Ok(ParsedResponse {
        body_xml: body_buf,
        is_fault,
        faultcode_hash: if is_fault { register_str(&faultcode) } else { 0 },
        faultstring_hash: if is_fault { register_str(&faultstring) } else { 0 },
    })
}

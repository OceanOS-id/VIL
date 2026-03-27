# 703-protocol-soap-client

SOAP client calling the public NumberConversion web service.

## What it shows

- `SoapClient::new()` with a remote SOAP endpoint
- `call_action()` sending a SOAP 1.1 envelope
- `ParsedResponse.body_xml` and `.is_fault` inspection
- `db_log!` auto-emitted (op_type=4 CALL) by `vil_soap` on every call
- `StdoutDrain::resolved()` output format

## Prerequisites

Network access to `https://www.dataaccess.com/webservicesserver/`.

No Docker required. If the endpoint is unreachable, the example exits gracefully.

To test with a local mock:

```bash
docker run -p 8080:8080 soapui/soapui-mock-service:latest
```

## Run

```bash
cargo run -p example-703-protocol-soap-client
```

"""
RAG Guardrail — PII Detector (Python WASM)

Detects and redacts PII: email, phone, NIK (Indonesian ID), credit card.
HIPAA-compliant for healthcare RAG pipelines.
"""

import json
import re
import sys


# PII patterns
PATTERNS = {
    "email": re.compile(r'[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}'),
    "phone": re.compile(r'(?:\+62|08)\d{8,12}'),
    "nik": re.compile(r'\b\d{16}\b'),
    "credit_card": re.compile(r'\b(?:\d{4}[- ]?){3}\d{4}\b'),
    "ip_address": re.compile(r'\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b'),
}


def detect_pii(text: str) -> dict:
    """Detect PII types present in text."""
    found = {}
    for pii_type, pattern in PATTERNS.items():
        matches = pattern.findall(text)
        if matches:
            found[pii_type] = len(matches)
    return found


def redact(text: str) -> str:
    """Redact all PII from text."""
    result = text
    for pii_type, pattern in PATTERNS.items():
        if pii_type == "email":
            result = pattern.sub("[EMAIL_REDACTED]", result)
        elif pii_type == "phone":
            result = pattern.sub("[PHONE_REDACTED]", result)
        elif pii_type == "nik":
            result = pattern.sub("[NIK_REDACTED]", result)
        elif pii_type == "credit_card":
            result = pattern.sub("[CC_REDACTED]", result)
        elif pii_type == "ip_address":
            result = pattern.sub("[IP_REDACTED]", result)
    return result


def process_one(raw):
    input_data = raw if isinstance(raw, dict) else (json.loads(raw) if str(raw).strip() else {})
    text = input_data.get("text", "")

    pii_found = detect_pii(text)
    has_pii = len(pii_found) > 0
    redacted_text = redact(text) if has_pii else text

    return ({
        "has_pii": has_pii,
        "pii_types": pii_found,
        "redacted_text": redacted_text,
        "original_length": len(text),
        "redacted_length": len(redacted_text),
        "status": "REDACTED" if has_pii else "CLEAN"
    })


# ── VIL Sidecar Dual-Mode: UDS+SHM primary, stdin/stdout fallback ──
try:
    import os
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../../../crates/vil_sidecar/sdk'))
    from vil_sidecar_sdk import SidecarApp
    _VIL_SDK = True
except ImportError:
    _VIL_SDK = False


# ── VIL Sidecar: 034 pattern (UDS+SHM primary, stdin/stdout fallback) ──
import os
try:
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../../../crates/vil_sidecar/sdk'))
    from vil_sidecar_sdk import SidecarApp
    VIL_SDK = True
except ImportError:
    VIL_SDK = False

if VIL_SDK and os.environ.get("VIL_SIDECAR_SOCKET"):
    app = SidecarApp("guardrail_pii_detector")
    app.handler("execute")(process_one)
    app.run()
else:
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            data = json.loads(line)
            result = process_one(data)
            print(json.dumps(result), flush=True)
        except Exception as e:
            print(json.dumps({"error": str(e)}), flush=True)

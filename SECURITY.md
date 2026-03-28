# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in VIL, please report it responsibly.

**Do NOT open a public GitHub issue for security vulnerabilities.**

### How to Report

1. **Email:** Send details to **security@vastar.id**
2. **Subject:** `[VIL Security] <brief description>`
3. **Include:**
   - Description of the vulnerability
   - Steps to reproduce
   - Affected crate(s) and version(s)
   - Potential impact assessment
   - Suggested fix (if any)

### What to Expect

- **Acknowledgment:** Within 48 hours
- **Initial assessment:** Within 5 business days
- **Fix timeline:** Critical issues within 7 days, High within 14 days
- **Disclosure:** Coordinated disclosure after fix is released

### Scope

The following are in scope:
- All `vil_*` crates published on crates.io
- Server framework security (authentication, authorization, encryption)
- Memory safety issues in unsafe code (vil_shm, vil_queue, vil_log, vil_rt)
- Shared memory isolation and access control
- WASM/capsule sandbox escapes
- Cryptographic implementation issues

### Out of Scope

- Vulnerabilities in third-party dependencies (report upstream, but let us know)
- Denial of service via expected resource consumption
- Issues requiring physical access to the server

### Recognition

We appreciate responsible disclosure. Contributors who report valid security issues will be acknowledged in the CHANGELOG (unless they prefer anonymity).

#!/usr/bin/env python3
"""
Generate template.toml for ALL examples in the VIL repo.
Analyzes Cargo.toml and src/main.rs to auto-detect:
- Package name, port, upstream URL
- Test requirements (simulator, credit-simulator, external services)
- HTTP endpoints for runtime testing
"""

import os, re, json

EXAMPLES_DIR = "examples"

# Known external dependencies by example prefix/name patterns
NEEDS_SIMULATOR = [
    "ai-gw", "vilapp-ai", "multi-model", "rag", "agent", "llm", "ab-testing",
    "sse-standard-dialect", "multi-pipeline"
]
NEEDS_CREDIT_SIM = ["credit-npl", "credit-quality", "credit-regulatory"]
NEEDS_EXTERNAL = {
    "mongo": "mongodb://localhost:27017",
    "clickhouse": "http://localhost:8123",
    "elastic": "http://localhost:9200",
    "rabbitmq": "amqp://localhost:5672",
    "sqs": "http://localhost:4566",
    "kafka": "localhost:9092",
    "nats": "nats://localhost:4222",
    "mqtt": "mqtt://localhost:1883",
    "s3": "http://localhost:9000",
    "postgres": "postgres://localhost:5432",
    "modbus": "localhost:502",
}

def detect_category(name):
    """Categorize example by number prefix."""
    num = name.split("-")[0]
    if num.endswith("b") or num.endswith("c"):
        num = num[:-1]
    try:
        n = int(num)
    except ValueError:
        return "other"
    if n < 100: return "basic"
    if n < 200: return "pipeline"
    if n < 300: return "llm"
    if n < 400: return "rag"
    if n < 500: return "agent"
    if n < 600: return "villog"
    if n < 700: return "storage"
    if n < 800: return "messaging"
    if n < 900: return "trigger"
    return "other"

def detect_test_type(name, main_rs):
    """Detect what the example needs to run."""
    name_lower = name.lower()

    # Credit simulator
    for pat in NEEDS_CREDIT_SIM:
        if pat in name_lower:
            return "credit-simulator", "cargo install credit-data-simulator && credit-data-simulator &"

    # AI endpoint simulator
    for pat in NEEDS_SIMULATOR:
        if pat in name_lower:
            return "simulator", "ai-endpoint-simulator &"

    # External services
    for svc, url in NEEDS_EXTERNAL.items():
        if svc in name_lower:
            return f"external:{svc}", f"# Requires {svc} at {url}"

    # VilLog examples (no server)
    cat = detect_category(name)
    if cat == "villog":
        return "standalone", "# No server needed — run binary directly"

    # Trigger examples
    if cat == "trigger":
        return "standalone", "# Trigger-based — run binary directly"

    # Default: standalone HTTP server
    return "standalone", ""

def detect_port(main_rs):
    """Extract port from source code."""
    # Look for .port(NNNN) pattern
    m = re.search(r'\.port\((\d{4})\)', main_rs)
    if m: return int(m.group(1))

    # Look for :NNNN in listen/bind
    m = re.search(r'(?:listen|bind).*?(\d{4})', main_rs)
    if m: return int(m.group(1))

    return 8080

def detect_upstream(main_rs):
    """Extract upstream URL from source code."""
    m = re.search(r'(?:UPSTREAM_URL|upstream).*?"(http[^"]+)"', main_rs)
    if m: return m.group(1)
    return ""

def detect_endpoints(main_rs):
    """Extract HTTP endpoints from source code."""
    endpoints = []
    # Look for .endpoint(Method::XXX, "/path", ...)
    for m in re.finditer(r'endpoint\(Method::(\w+),\s*"(/[^"]*)"', main_rs):
        method = m.group(1)
        path = m.group(2)
        if path.startswith("/_vil") or path == "/favicon.ico":
            continue
        endpoints.append((method, path))

    # Look for .route("/path", get/post(...))
    for m in re.finditer(r'\.route\("(/[^"]*)",\s*(get|post|put|delete)', main_rs):
        path = m.group(1)
        method = m.group(2).upper()
        if path.startswith("/_vil"):
            continue
        endpoints.append((method, path))

    return endpoints

def detect_service_prefix(main_rs):
    """Detect ServiceProcess prefix."""
    m = re.search(r'ServiceProcess::new\("(\w+)"\)', main_rs)
    if m: return m.group(1)
    return ""

def generate_title(name):
    """Generate human-readable title from example directory name."""
    # Remove number prefix
    parts = name.split("-", 1)
    if len(parts) > 1:
        title = parts[1]
    else:
        title = name
    # Remove "basic-" prefix
    title = re.sub(r'^basic-', '', title)
    # Title case
    title = title.replace("-", " ").title()
    return title

def generate_template_toml(example_dir):
    """Generate template.toml content for an example."""
    name = os.path.basename(example_dir)
    cargo_path = os.path.join(example_dir, "Cargo.toml")
    main_path = os.path.join(example_dir, "src/main.rs")

    if not os.path.exists(cargo_path):
        return None

    cargo = open(cargo_path).read()
    main_rs = open(main_path).read() if os.path.exists(main_path) else ""

    # Extract package name
    m = re.search(r'^name\s*=\s*"([^"]+)"', cargo, re.MULTILINE)
    pkg_name = m.group(1) if m else name

    # Detect fields
    port = detect_port(main_rs)
    upstream = detect_upstream(main_rs)
    category = detect_category(name)
    test_type, prereq = detect_test_type(name, main_rs)
    endpoints = detect_endpoints(main_rs)
    prefix = detect_service_prefix(main_rs)
    title = generate_title(name)

    # Generate description from first comment block
    desc_match = re.search(r'//\s*║\s*\d+\s*—\s*(.+?)║', main_rs)
    if desc_match:
        desc = desc_match.group(1).strip()
    else:
        desc = title

    # Build test section
    test_lines = []
    test_lines.append(f'type = "{test_type}"')
    if prereq:
        test_lines.append(f'prereq = "{prereq}"')
    test_lines.append(f'port = {port}')

    if endpoints:
        method, path = endpoints[0]
        full_path = f"/api/{prefix}{path}" if prefix and not path.startswith(f"/api/{prefix}") else path
        # Simplify: if endpoint has ServiceProcess prefix, path is already /api/prefix/path
        # Check if main_rs has the full path pattern
        if prefix and f'"/api/{prefix}' not in main_rs and f'"/{prefix}' not in main_rs:
            full_path = f"/api/{prefix}{path}"
        else:
            full_path = path

        test_lines.append(f'method = "{method}"')
        test_lines.append(f'path = "{full_path}"')

        if method == "POST":
            if "prompt" in main_rs.lower() or upstream:
                test_lines.append('body = \'{"prompt":"test"}\'')
            elif "task" in main_rs.lower() or "crud" in name:
                test_lines.append('body = \'{"title":"test","done":false}\'')
            else:
                test_lines.append('body = \'{"test":true}\'')
        test_lines.append('expect_status = 200')
    elif category == "villog":
        test_lines.append('method = "RUN"')
        test_lines.append('expect_exit = 0')
    else:
        # No endpoints detected — just verify it starts
        test_lines.append('method = "GET"')
        test_lines.append('path = "/health"')
        test_lines.append('expect_status = 200')

    # Build replace section
    replace_lines = [f'package_name = "{pkg_name}"']
    replace_lines.append(f'port = "{port}"')
    if upstream:
        replace_lines.append(f'upstream = "{upstream}"')

    # Compose TOML
    toml = f"""[template]
id = "{name}"
title = "{title}"
description = "{desc}"
default_port = {port}
default_upstream = "{upstream}"
category = "{category}"

[template.replace]
{chr(10).join(replace_lines)}

[template.test]
{chr(10).join(test_lines)}
"""
    return toml

def list_files(example_dir):
    """List all files in example (excluding target/ and template.toml)."""
    files = []
    for root, dirs, fnames in os.walk(example_dir):
        dirs[:] = [d for d in dirs if d not in ("target", ".git")]
        for f in fnames:
            if f == "template.toml":
                continue
            rel = os.path.relpath(os.path.join(root, f), example_dir)
            files.append(rel)
    return sorted(files)

def generate_index(examples_dir):
    """Generate template-index.json from all template.toml files."""
    templates = []
    for entry in sorted(os.listdir(examples_dir)):
        toml_path = os.path.join(examples_dir, entry, "template.toml")
        if not os.path.isfile(toml_path):
            continue

        # Simple TOML parser
        data = {}
        replace = {}
        test = {}
        section = "template"
        for line in open(toml_path):
            line = line.strip()
            if line.startswith("[template.replace]"):
                section = "replace"; continue
            if line.startswith("[template.test]"):
                section = "test"; continue
            if line.startswith("["):
                section = line.strip("[]"); continue
            m = re.match(r"(\w+)\s*=\s*'([^']*)'", line) or re.match(r'(\w+)\s*=\s*"([^"]*)"', line)
            if not m:
                m2 = re.match(r'(\w+)\s*=\s*(\d+)', line)
                if m2:
                    target = data if section == "template" else test if section == "test" else replace
                    target[m2.group(1)] = int(m2.group(2))
                continue
            target = data if section == "template" else test if section == "test" else replace
            target[m.group(1)] = m.group(2)

        files = list_files(os.path.join(examples_dir, entry))

        templates.append({
            "id": data.get("id", entry),
            "title": data.get("title", entry),
            "description": data.get("description", ""),
            "default_port": data.get("default_port", 8080),
            "default_upstream": data.get("default_upstream", ""),
            "category": data.get("category", "basic"),
            "example_dir": entry,
            "replace": replace,
            "test": test,
            "files": files,
        })

    return {"version": 2, "templates": templates}


if __name__ == "__main__":
    import sys

    if len(sys.argv) > 1 and sys.argv[1] == "--index":
        # Generate template-index.json
        index = generate_index(EXAMPLES_DIR)
        print(json.dumps(index, indent=2))
        sys.exit(0)

    # Generate template.toml for all examples
    count = 0
    skip = 0
    for entry in sorted(os.listdir(EXAMPLES_DIR)):
        example_dir = os.path.join(EXAMPLES_DIR, entry)
        if not os.path.isdir(example_dir):
            continue

        toml_path = os.path.join(example_dir, "template.toml")

        # Skip if already has template.toml (don't overwrite manual ones)
        if os.path.exists(toml_path):
            skip += 1
            continue

        content = generate_template_toml(example_dir)
        if content:
            with open(toml_path, "w") as f:
                f.write(content)
            count += 1
            print(f"  + {entry}")
        else:
            print(f"  - {entry} (no Cargo.toml)")

    print(f"\nGenerated: {count}, Skipped (existing): {skip}")

#!/usr/bin/env bash
# =============================================================================
# VIL — Package Ready-to-Upload Workflow Samples
# =============================================================================
# Produces .tar.gz files under releases/ that are uploadable to vil-server
# via 2 curl commands (WASM + YAML). No cargo, no git clone needed.
#
# Output structure inside each tarball:
#   sample/
#     README.md             — 3-curl demo for this sample
#     <workflow>.yaml       — VWFD definition
#     <module>.wasm         — pre-compiled WASM handler(s)
#     curl-upload.sh        — convenience script that issues the 2 uploads
#
# Usage:
#   ./scripts/package-samples.sh                   # package all known samples
#   ./scripts/package-samples.sh hello-server      # package one
#
# After running, upload the resulting .tar.gz files as release assets on
# github.com/OceanOS-id/VIL/releases/tag/v0.4.0.
# =============================================================================

set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="0.4.0"
OUT_DIR="releases"
mkdir -p "$OUT_DIR"

# ─────────────────────────────────────────────────────────────────────────────
# Registry of samples — each row:
#   <slug>|<example-dir>|<webhook-path>|<upstream-name>|<upstream-install-cmd>|<upstream-start-cmd>|<test-payload>
#
# Keep short: these drive the 60-second demo on the website + Docker Hub.
# <upstream-name>        = `-` if the workflow has no external dependency.
# <upstream-install-cmd> = how the user installs the upstream (shown in README).
# <upstream-start-cmd>   = how the user starts the upstream (shown in README).
# ─────────────────────────────────────────────────────────────────────────────
SAMPLES=(
  "ai-gateway|examples/001-basic-ai-gw-demo/vwfd|/trigger|ai-endpoint-simulator|cargo install ai-endpoint-simulator|ai-endpoint-simulator &|{\"prompt\":\"hello\"}"
)

# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

package_sample() {
  local slug="$1"
  local src_dir="$2"
  local hook_path="$3"
  local upstream_name="${4:-}"
  local upstream_install="${5:-}"
  local upstream_start="${6:-}"
  local test_payload="${7:-}"
  [[ -z "$test_payload" ]] && test_payload='{}'

  echo ""
  echo "▸ Packaging sample: $slug"
  echo "  Source: $src_dir"

  if [[ ! -d "$src_dir" ]]; then
    echo "  ✗ Source directory missing: $src_dir"
    return 1
  fi

  # Staging area inside the tarball
  local work
  work=$(mktemp -d)
  local stage="$work/sample"
  mkdir -p "$stage"

  # 1. Copy YAML workflows (entire workflows/ dir flattened)
  local yaml_count=0
  if [[ -d "$src_dir/workflows" ]]; then
    for y in "$src_dir/workflows"/*.yaml "$src_dir/workflows"/*.yml; do
      [[ -f "$y" ]] || continue
      cp "$y" "$stage/"
      yaml_count=$((yaml_count + 1))
    done
  fi
  if [[ $yaml_count -eq 0 ]]; then
    echo "  ✗ No YAML workflows in $src_dir/workflows"
    rm -rf "$work"
    return 1
  fi

  # 2. Copy every .wasm under $src_dir (any language subdir — rust/, assemblyscript/, etc.)
  # WASM is OPTIONAL — pure-Connector workflows work without any .wasm.
  local wasm_count=0
  while IFS= read -r -d '' w; do
    cp "$w" "$stage/"
    wasm_count=$((wasm_count + 1))
  done < <(find "$src_dir" -type f -name '*.wasm' -print0 2>/dev/null || true)

  # 3. Build the curl-upload.sh convenience script
  local module_refs=()
  for w in "$stage"/*.wasm; do
    local mref
    mref=$(basename "$w" .wasm)
    module_refs+=("$mref")
  done

  cat > "$stage/curl-upload.sh" <<'UPLOAD_SH'
#!/usr/bin/env bash
# Upload this sample to a running vil-server (default: http://localhost:3080).
# Usage:
#   ./curl-upload.sh                                    # localhost, no auth
#   ./curl-upload.sh http://vil-server.example.com      # custom host
#   ADMIN_KEY=secret ./curl-upload.sh                   # with auth
set -euo pipefail

HOST="${1:-http://localhost:3080}"
AUTH=()
if [[ -n "${ADMIN_KEY:-}" ]]; then
  AUTH=(-H "X-Admin-Key: ${ADMIN_KEY}")
fi

cd "$(dirname "$0")"

echo "▸ Uploading to $HOST"

for w in *.wasm; do
  [[ -f "$w" ]] || continue
  ref="${w%.wasm}"
  echo "  WASM : $ref"
  curl -fsS -X POST "$HOST/api/admin/upload/wasm" \
    "${AUTH[@]}" \
    -H "X-Module-Ref: $ref" \
    --data-binary "@$w" | head -c 200 ; echo
done

for y in *.yaml *.yml; do
  [[ -f "$y" ]] || continue
  echo "  YAML : $(basename "$y")"
  curl -fsS -X POST "$HOST/api/admin/upload" \
    "${AUTH[@]}" \
    -H 'Content-Type: application/x-yaml' \
    --data-binary "@$y" | head -c 200 ; echo
done

echo ""
echo "✓ All uploaded. Hit your endpoint at $HOST/<webhook_path>"
UPLOAD_SH
  chmod +x "$stage/curl-upload.sh"

  # 4. Build the sample README.md (60-second demo)
  local upstream_block=""
  local teardown_extra=""
  if [[ -n "$upstream_name" && "$upstream_name" != "-" ]]; then
    upstream_block="# 0. (prerequisite) Install + start the upstream this workflow talks to:
#    ${upstream_name} — binary is on crates.io
${upstream_install}
${upstream_start}

"
    teardown_extra=$'\npkill -f '"${upstream_name}"
  fi

  local wasm_block=""
  local contents_wasm=""
  if [[ $wasm_count -gt 0 ]]; then
    contents_wasm=$(for w in "$stage"/*.wasm; do
      n=$(basename "$w")
      size=$(du -h "$w" | awk '{print $1}')
      echo "| \`$n\` | Pre-compiled WASM handler ($size) |"
    done)
  fi

  local contents_yaml
  contents_yaml=$(for y in "$stage"/*.yaml "$stage"/*.yml; do
    [[ -f "$y" ]] || continue
    n=$(basename "$y")
    echo "| \`$n\` | VWFD workflow definition |"
  done)

  local opening
  if [[ -n "$upstream_name" && "$upstream_name" != "-" ]]; then
    opening="Ready-to-upload workflow for a running \`vil-server\` — YAML + WASM pre-built. This sample talks to an upstream (\`${upstream_name}\`), install shown below."
  else
    opening="Ready-to-upload workflow for a running \`vil-server\`. No cargo, no compile, no git clone — everything pre-built."
  fi

  cat > "$stage/README.md" <<SAMPLE_README
# VIL Sample — $slug (v$VERSION)

${opening}

## 60-Second Demo

\`\`\`bash
${upstream_block}# 1. Start the provisionable VIL server
docker run -d --network host --name vil vilfounder/vil:${VERSION}

# 2. Upload this sample (all workflows + any WASM modules)
./curl-upload.sh

# 3. Hit the endpoint
curl -X POST http://localhost:3080${hook_path} \\
  -H 'Content-Type: application/json' \\
  -d '${test_payload}'
\`\`\`

## Contents

| File | Role |
|------|------|
${contents_yaml}
${contents_wasm}
| \`curl-upload.sh\` | One-shot upload script (uploads YAML + any WASM modules via admin API) |
| \`README.md\` | This file |

## Activity Types in This Sample

$(grep -h 'activity_type:' "$stage"/*.yaml "$stage"/*.yml 2>/dev/null | sed -E 's/^[[:space:]]*(-[[:space:]]+)?activity_type:[[:space:]]*/- `/; s/[[:space:]]*$/`/' | sort -u)

## Teardown

\`\`\`bash
docker rm -f vil${teardown_extra}
\`\`\`

## License

This sample is distributed alongside VIL source under the same licensing terms — library code is Apache 2.0 / MIT, VIL workflow runtime is VSAL. See [LICENSING.md](https://github.com/OceanOS-id/VIL/blob/main/LICENSING.md) — internal business use is free; operating as a multi-tenant Workflow-as-a-Service requires a commercial agreement with Vastar.
SAMPLE_README

  # 5. Pack the tarball
  local out_file="$OUT_DIR/sample-${slug}.tar.gz"
  tar -czf "$out_file" -C "$work" sample

  local size
  size=$(du -h "$out_file" | awk '{print $1}')
  echo "  ✓ $out_file ($size) — $yaml_count workflow(s), $wasm_count WASM module(s)"

  rm -rf "$work"
}

# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────
echo "═══════════════════════════════════════════════════════════════"
echo "  VIL Sample Packaging — v$VERSION"
echo "  Output: $OUT_DIR/"
echo "═══════════════════════════════════════════════════════════════"

if [[ $# -gt 0 ]]; then
  # Filter to requested slugs
  FILTER="$*"
else
  FILTER=""
fi

count=0
fail=0
for row in "${SAMPLES[@]}"; do
  IFS='|' read -r slug src_dir hook_path upstream_name upstream_install upstream_start test_payload <<<"$row"
  if [[ -n "$FILTER" ]] && ! [[ " $FILTER " =~ " $slug " ]]; then
    continue
  fi
  if package_sample "$slug" "$src_dir" "$hook_path" "$upstream_name" "$upstream_install" "$upstream_start" "$test_payload"; then
    count=$((count + 1))
  else
    fail=$((fail + 1))
  fi
done

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  Done. Packaged: $count · Failed: $fail"
echo ""
if [[ $count -gt 0 ]]; then
  echo "  Upload as release assets:"
  echo "    gh release upload v$VERSION $OUT_DIR/*.tar.gz"
  echo ""
  echo "  Or create the release + upload in one go:"
  echo "    gh release create v$VERSION $OUT_DIR/*.tar.gz \\"
  echo "      --title 'VIL v$VERSION' --notes-file CHANGELOG.md"
fi
echo "═══════════════════════════════════════════════════════════════"

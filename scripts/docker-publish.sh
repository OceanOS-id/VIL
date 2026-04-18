#!/usr/bin/env bash
# =============================================================================
# VIL — Docker Hub Publish Script (multi-arch)
# =============================================================================
# Publishes vilfounder/vil:{version,minor,latest} to Docker Hub for linux/amd64
# and linux/arm64.
#
# Prereqs:
#   1. docker buildx installed (v0.31.1+ on this machine — OK)
#   2. QEMU user-mode emulation for cross-building (see OPTIONAL setup below)
#   3. docker login done interactively (uses your Docker Hub credentials)
#
# Usage:
#   ./scripts/docker-publish.sh             # full pipeline: login → build → push
#   ./scripts/docker-publish.sh --build-only # build without pushing
#
# License note: this image contains VSAL-licensed software. Before publishing,
# ensure the Docker Hub repository's long description has been updated with the
# VSAL + WaaS warning (see docker/DOCKER_HUB_README.md).
# =============================================================================

set -euo pipefail

VERSION="0.4.0"
MINOR="0.4"
REPO="vilfounder/vil"

BUILD_ONLY=false
if [[ "${1:-}" == "--build-only" ]]; then
  BUILD_ONLY=true
fi

# ─────────────────────────────────────────────────────────────────────────────
# OPTIONAL setup (run once on a fresh machine)
# ─────────────────────────────────────────────────────────────────────────────
#
# If building for linux/arm64 on an amd64 host, you need QEMU:
#   docker run --privileged --rm tonistiigi/binfmt --install all
#
# Create a buildx builder that supports multi-arch:
#   docker buildx create --name vil-builder --driver docker-container --bootstrap
#   docker buildx use vil-builder
#
# Verify it supports both platforms:
#   docker buildx inspect --bootstrap | grep Platforms
#
# ─────────────────────────────────────────────────────────────────────────────

cd "$(dirname "$0")/.."

echo "═══════════════════════════════════════════════════════════════"
echo "  VIL Docker Publish — ${REPO}:${VERSION}"
echo "  Platforms: linux/amd64, linux/arm64"
echo "═══════════════════════════════════════════════════════════════"

# Confirm builder exists and supports multi-arch
if ! docker buildx inspect --bootstrap 2>&1 | grep -qE "linux/amd64.*linux/arm64|linux/arm64.*linux/amd64"; then
  echo ""
  echo "⚠ The current buildx builder does not support both linux/amd64 and linux/arm64."
  echo "  Run these once, then re-run this script:"
  echo "    docker run --privileged --rm tonistiigi/binfmt --install all"
  echo "    docker buildx create --name vil-builder --driver docker-container --bootstrap"
  echo "    docker buildx use vil-builder"
  echo ""
  exit 1
fi

if [[ "$BUILD_ONLY" == "false" ]]; then
  # Confirm login — check for config file with auths
  if ! grep -q '"auths"' "${HOME}/.docker/config.json" 2>/dev/null; then
    echo ""
    echo "⚠ Not logged in to Docker Hub. Run:"
    echo "    docker login"
    echo ""
    exit 1
  fi
fi

echo ""
echo "▸ Building (this will take 5-10 min per arch if no cache)..."
echo ""

PUSH_FLAG="--push"
[[ "$BUILD_ONLY" == "true" ]] && PUSH_FLAG="--load"

docker buildx build \
  --platform linux/amd64,linux/arm64 \
  --tag "${REPO}:${VERSION}" \
  --tag "${REPO}:${MINOR}" \
  --tag "${REPO}:latest" \
  --label "org.opencontainers.image.created=$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --label "org.opencontainers.image.revision=$(git rev-parse HEAD 2>/dev/null || echo unknown)" \
  ${PUSH_FLAG} \
  .

echo ""
echo "═══════════════════════════════════════════════════════════════"
if [[ "$BUILD_ONLY" == "true" ]]; then
  echo "  ✓ Built locally (no push)"
else
  echo "  ✓ Pushed to Docker Hub"
  echo ""
  echo "  Next: update the Docker Hub long description."
  echo "    - Log in to https://hub.docker.com/r/${REPO}"
  echo "    - Click 'Manage Repository' → Description"
  echo "    - Paste contents of docker/DOCKER_HUB_README.md"
  echo ""
  echo "  Verify:"
  echo "    docker pull ${REPO}:${VERSION}"
  echo "    docker run -d -p 3080:3080 --name vil ${REPO}:${VERSION}"
  echo "    curl http://localhost:3080/api/admin/health"
fi
echo "═══════════════════════════════════════════════════════════════"

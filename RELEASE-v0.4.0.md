# VIL v0.4.0 — Release Checklist

**Maintainer:** tick each box as you complete it. Items are ordered so that earlier verifications gate later ones.

---

## Phase 1 — Verify Local State

- [ ] `git status` clean on both repos (VIL + website) OR intentional WIP noted
- [ ] `cargo check --workspace` passes in VIL repo
- [ ] Website `npm run build` passes (no TS errors, no missing i18n keys)
- [ ] Docker image built locally: `docker images vilfounder/vil:0.4.0` shows ~180 MB
- [ ] End-to-end sanity test:
  - [ ] `ai-endpoint-simulator &` → listens on :4545
  - [ ] `docker run -d --network host --name vil vilfounder/vil:0.4.0` → healthy
  - [ ] `tar xzf releases/sample-ai-gateway.tar.gz && cd sample && ./curl-upload.sh` succeeds
  - [ ] `curl -N POST :3080/trigger -d '{"prompt":"hello"}'` streams SSE
  - [ ] Cleanup: `docker rm -f vil && pkill -f ai-endpoint-simulator`

## Phase 2 — Commit Everything Locally

**VIL repo** (`/home/abraham/Prdmid/vil-project/vil`):

Already stacked (no action):
- [ ] `dd0e32c release: v0.4.0 — licensing restructure + bench infra + vastar sweep`
- [ ] `b14a255 docs(license): formalize Licensor Reserved Rights for Vastar commercial moat`
- [ ] `8b4fae9 docs(license): reinforce Significant Business Process Exception`
- [ ] `bc7c651 docs(license): complete VSAL as a Sustainable Use License (SUL)`
- [ ] `33da28b fix(examples): 010 vwfd test syntax + 205 SSE response passthrough`

New (uncommitted — stage + commit before push):
- [ ] Docker + tooling — `Dockerfile`, `Dockerfile.slim`, `docker-compose.yml`, `docker/DOCKER_HUB_README.md`, `scripts/docker-publish.sh`, `scripts/package-samples.sh`, `.dockerignore`, `.github/workflows/release-samples.yml`, `releases/sample-ai-gateway.tar.gz`
- [ ] Docs — `CHANGELOG.md`, `RELEASE-v0.4.0.md`

Suggested commit (one):
```bash
git add Dockerfile Dockerfile.slim docker-compose.yml docker/ scripts/docker-publish.sh \
        scripts/package-samples.sh .dockerignore .github/workflows/release-samples.yml \
        releases/ CHANGELOG.md RELEASE-v0.4.0.md
git commit -m "release(0.4.0): Docker image + sample bundles + CHANGELOG + release checklist"
```

**Website repo** (`/home/abraham/Vastar-Publication/website-vastar-id`):

Uncommitted changes:
- [ ] Locale stripping (ar/ja/tr removed)
- [ ] VIL.tsx rewrite (60s demo, Polyglot matrix, Provisionable section, Licensing tiers, VSAL + WaaS banner)
- [ ] docs updates (provisionable-workflow.md new, wasm-sidecar.md, transpile-sdk.md)
- [ ] Image optimization (WebP, 38MB → 3.2MB)
- [ ] Route lazy-loading + Vite manualChunks
- [ ] Dead deps removed (moment, lucide-react, tsparticles)

Suggested commit:
```bash
git add -A
git commit -m "release(0.4.0): website overhaul — VSAL prominence, Docker Quick Start, polyglot matrix, perf optimization"
```

## Phase 3 — Tag + Push GitHub

```bash
# VIL repo
cd ~/Prdmid/vil-project/vil
git tag -a v0.4.0 -m "VIL v0.4.0 — licensing restructure + Docker + provisionable"
git push origin main
git push origin v0.4.0          # triggers .github/workflows/release-samples.yml

# Website repo
cd ~/Vastar-Publication/website-vastar-id
git push origin main
```

- [ ] Both pushes succeed
- [ ] `release-samples.yml` workflow run passes (check Actions tab on GitHub)
- [ ] Release assets visible at `https://github.com/OceanOS-id/VIL/releases/tag/v0.4.0`
- [ ] `releases/sample-ai-gateway.tar.gz` attached to the release

## Phase 4 — GitHub Release Body

Manually create the release or update the auto-created one:

- [ ] Title: `VIL v0.4.0 — Source-Available Workflow Runtime`
- [ ] Body: paste from `CHANGELOG.md` `[0.4.0]` section
- [ ] Assets: `sample-ai-gateway.tar.gz` (from release-samples workflow)
- [ ] Mark as latest release

## Phase 5 — Docker Hub Push

```bash
cd ~/Prdmid/vil-project/vil

# One-time setup (skip if already done)
docker run --privileged --rm tonistiigi/binfmt --install all
docker buildx use vil-builder || docker buildx create --name vil-builder --driver docker-container --bootstrap --use

# Login
docker login                      # as vilfounder

# Build + push debian-slim (default)
./scripts/docker-publish.sh --build-only    # dry run multi-arch, verify
./scripts/docker-publish.sh                 # publish

# Build + push distroless variant
# (manual buildx command — may need a second script variant for this)
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f Dockerfile.slim \
  --tag vilfounder/vil:0.4.0-slim \
  --tag vilfounder/vil:0.4-slim \
  --tag vilfounder/vil:slim \
  --push .
```

- [ ] `docker pull vilfounder/vil:0.4.0` works from a fresh machine
- [ ] `docker pull vilfounder/vil:0.4.0-slim` works
- [ ] Both pulls return healthy on `/api/admin/health`

## Phase 6 — Docker Hub Description

- [ ] Log in to https://hub.docker.com/r/vilfounder/vil
- [ ] Manage Repository → Description → paste contents of `docker/DOCKER_HUB_README.md`
- [ ] Verify VSAL + WaaS warning renders correctly at the top
- [ ] Short description field: "VIL Provisionable Workflow Runtime — VSAL. Hot-upload workflows via admin API. Mix Rust + WASM (4 langs) + sidecar (9 langs) handlers."

## Phase 7 — Publish Apache/MIT Crates to crates.io

```bash
cd ~/Prdmid/vil-project/vil
cargo login                        # one-time
./scripts/publish-all.sh --dry-run # ALL 165 Apache/MIT crates dry-run
./scripts/publish-all.sh           # live publish
```

- [ ] `scripts/publish-all.sh` SKIP_CRATES list includes all 7 VSAL crates (verify before running)
- [ ] All Apache/MIT crates publish successfully (rate-limits may need retry — script handles this)
- [ ] Spot-check on crates.io: `vil_server`, `vil_expr`, `vil_db_sqlx` all show 0.4.0

## Phase 8 — Website Deployment

- [ ] Website production rebuild picks up new commit
- [ ] Visit `https://vastar.id/products/vil` — 60s demo renders correctly, image WebP loads fast
- [ ] Visit `https://vastar.id/docs/vil/guides/provisionable-workflow` — new page renders
- [ ] LCP check: main hero image <1s on Lighthouse (post image-optimization)

## Phase 9 — Post-Launch Verification

From a clean environment (new VM / Docker Playground / CI runner):

```bash
# The actual 60-second demo users will run — verify it works end-to-end
cargo install ai-endpoint-simulator
ai-endpoint-simulator &
docker run -d --network host --name vil vilfounder/vil:0.4.0
curl -sSL https://github.com/OceanOS-id/VIL/releases/download/v0.4.0/sample-ai-gateway.tar.gz | tar xz
cd sample && ./curl-upload.sh
curl -N -X POST http://localhost:3080/trigger -H 'Content-Type: application/json' -d '{"prompt":"hello"}'
```

- [ ] Every step above works without error
- [ ] Token count / response content reasonable
- [ ] No warnings or errors in Docker logs

## Phase 10 — Announcement (optional, Anda decide timing)

- [ ] Draft blog post (see `docs/blog/v0.4.0.md` or wherever)
- [ ] Twitter / X thread with Quick Start GIF
- [ ] LinkedIn post (Indonesian tech community)
- [ ] dev.to / HackerNews Show HN submission
- [ ] Vastar Slack / Discord announcement (if exists)

---

## Rollback Plan (if needed)

If a critical bug surfaces after Docker Hub publish but before wide announcement:

```bash
# Docker Hub — untag latest, keep 0.4.0 explicit tag
docker buildx imagetools create --tag vilfounder/vil:latest vilfounder/vil:0.3.0

# crates.io — yank affected crates (keeps history, blocks new dependents)
cargo yank --vers 0.4.0 <crate-name>
```

Fix → release as `0.4.1` → re-run Phase 5 onwards.

---

**Dates**:
- Planned: 2026-04-18
- Actual: _____________

**Sign-off**: _____________

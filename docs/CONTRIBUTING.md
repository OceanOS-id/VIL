# Contributing to VIL

Thank you for your interest in contributing to VIL! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please be respectful and constructive in all interactions.

---

## Getting Started

### 1. Fork and Clone

```bash
# Fork on GitHub, then clone your fork
git clone https://github.com/your-username/VIL.git
cd VIL

# Add upstream remote
git remote add upstream https://github.com/OceanOS-id/VIL.git
```

### 2. Create a Branch

```bash
# Always create a feature branch from main
git fetch upstream
git checkout -b feature/your-feature-name upstream/main
```

### 3. Set Up Development Environment

```bash
# Install dependencies
cargo build --workspace

# Run tests to verify setup
cargo test --workspace

# Install development tools
rustup component add rustfmt clippy
```

---

## Development Workflow

### Before You Start

1. **Check existing issues** — Avoid duplicate work
2. **Open an issue** for significant changes — Get feedback early
3. **Discuss in PRs** — We're here to help

### Code Style

We follow Rust conventions with these additions:

#### Formatting
```bash
# Format your code
cargo fmt --all

# Verify formatting
cargo fmt --all -- --check
```

#### Linting
```bash
# Check with Clippy
cargo clippy --workspace -- -D warnings

# Fix common issues automatically
cargo clippy --workspace --fix --allow-dirty
```

#### Documentation
```bash
# Generate and review docs
cargo doc --workspace --no-deps --open

# Add doc comments to public items
/// This is a public function that does something.
///
/// # Arguments
///
/// * `arg` - Description of arg
///
/// # Returns
///
/// A result or value
pub fn my_function(arg: &str) -> Result<()> {
    // ...
}
```

### Commit Messages

Follow conventional commits:

```
type(scope): description

Optional body explaining the change in more detail.

Fixes #123  (if closing an issue)
```

Types:
- `feat` — New feature
- `fix` — Bug fix
- `docs` — Documentation
- `test` — Add/modify tests
- `refactor` — Code refactoring
- `perf` — Performance improvement
- `chore` — Build, CI, etc.

Examples:
```
feat(vil_rt): add process lifecycle support
fix(vil_shm): prevent double-free in compaction
docs(developer-guide): clarify lane semantics
test(vil_validate): add edge cases for type checking
```

---

## Testing

### Run All Tests

```bash
cargo test --workspace --release
```

### Test Specific Crate

```bash
cargo test -p vil_rt --release
```

### With Output

```bash
cargo test --workspace -- --nocapture
```

### Integration Tests

```bash
# Run integration tests (examples)
cargo run --example semantic_types_demo --release
cargo run --example camera_pipeline --release
cargo run --example distributed_topo_demo --release
```

### Add New Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_your_feature() {
        // Arrange
        let input = setup_test_data();
        
        // Act
        let result = your_function(input);
        
        // Assert
        assert_eq!(result, expected_output);
    }

    #[test]
    #[should_panic(expected = "error message")]
    fn test_error_case() {
        // Test error handling
    }
}
```

### Benchmarks

For performance-critical code:

```bash
cargo bench --workspace
```

Add benchmarks in `benches/` directory using `criterion`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_feature(c: &mut Criterion) {
    c.bench_function("my_feature", |b| {
        b.iter(|| {
            your_function(black_box(test_input))
        })
    });
}

criterion_group!(benches, benchmark_feature);
criterion_main!(benches);
```

---

## Pull Request Process

### Before Submitting

1. **Rebase on latest main**
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run full test suite**
   ```bash
   cargo test --workspace --release
   cargo clippy --workspace -- -D warnings
   cargo fmt --all -- --check
   cargo doc --workspace --no-deps
   ```

3. **Run examples**
   ```bash
   cargo run --example semantic_types_demo --release
   cargo run --example vil_v2_full_demo --release
   ```

### Create the PR

1. **Push your branch**
   ```bash
   git push origin feature/your-feature-name
   ```

2. **Open PR on GitHub**
   - Use clear title (follows conventional commits)
   - Reference related issues: `Fixes #123`
   - Explain what changed and why

3. **PR Template**
   ```markdown
   ## Description
   Brief description of the changes
   
   ## Related Issue
   Fixes #123
   
   ## Type of Change
   - [ ] Bug fix
   - [ ] New feature
   - [ ] Breaking change
   - [ ] Documentation
   
   ## Testing
   - [ ] Added tests
   - [ ] Verified existing tests pass
   - [ ] Ran examples successfully
   
   ## Checklist
   - [ ] Code follows style guidelines
   - [ ] Documentation updated
   - [ ] No breaking changes (or documented)
   ```

### PR Review

- Respond to feedback promptly
- Push additional commits (don't force-push after review starts)
- Mark conversations as resolved once addressed

---

## Architecture & Design

### Understanding the Codebase

**Layer Structure:**
```
VIL v2 Semantic Superlayer
    ↓
VIL Runtime Substrate
    ↓
Rust + OS
```

**Key Directories:**
- `crates/vil_rt/` — Runtime kernel (core)
- `crates/vil_shm/` — Shared memory allocator
- `crates/vil_queue/` — Zero-copy queue
- `crates/vil_registry/` — Routing registry
- `crates/vil_ir/` — IR and contract generation
- `crates/vil_macros/` — Derive macros
- `crates/vil_validate/` — Validation passes
- `examples/` — Example pipelines
- `docs/` — User documentation

### Before Making Changes

1. **Read ARCHITECTURE_OVERVIEW.md** — Understand the design
2. **Study existing code** — Follow established patterns
3. **Check issue discussions** — Understand design intent

---

## Specific Contribution Types

### Bug Reports

**Title:** `bug: short description`

Include:
- Minimal reproducible example
- Expected vs actual behavior
- OS and Rust version (`rustc --version`)
- Relevant logs or output

### Feature Requests

**Title:** `feat: short description`

Include:
- Use case and motivation
- Proposed API/syntax
- Alternatives considered
- Potential impact on existing code

### Documentation

Changes to `docs/`:

```bash
# Edit markdown
vim docs/vil/VIL-Developer-Guide.md

# Verify builds correctly
cargo doc --workspace --no-deps
```

### Examples

Add new examples to `examples/` directory:

```
examples/my_new_feature/
├── Cargo.toml
├── src/
│   └── main.rs
└── README.md
```

Include in top-level `Cargo.toml`:
```toml
[[example]]
name = "my_new_feature"
path = "examples/my_new_feature/src/main.rs"
```

---

## Release Process (Maintainers)

1. **Update version** in all `Cargo.toml` files
2. **Update CHANGELOG.md**
3. **Create git tag:** `git tag v2.0.0`
4. **Push tag:** `git push upstream v2.0.0`
5. **Publish to crates.io:** `cargo publish --all`

---

## Common Issues & Solutions

### Issue: "tests fail locally but pass in CI"

```bash
# Clean and rebuild
cargo clean
cargo build --workspace

# Run tests again
cargo test --workspace --release
```

### Issue: "SHM issues during testing"

```bash
# Increase shared memory
sudo mount -o remount,size=4G /dev/shm

# Or run single test
cargo test --workspace -- --test-threads=1
```

### Issue: "Clippy warnings"

```bash
# See all warnings
cargo clippy --workspace

# Apply fixes automatically
cargo clippy --workspace --fix --allow-dirty
```

---

## Resources

- **GitHub**: https://github.com/OceanOS-id/VIL
- **Issues**: https://github.com/OceanOS-id/VIL/issues
- **Discussions**: https://github.com/OceanOS-id/VIL/discussions
- **Documentation**: [docs/](./)
- **Rust Book**: https://doc.rust-lang.org/book/

---

## Questions?

- Open a [GitHub Discussion](https://github.com/OceanOS-id/VIL/discussions)
- Ask in related issue
- Check existing documentation

---

## Thank You!

We appreciate your contributions to making VIL better. Whether it's code, documentation, examples, or bug reports—all help is valued.

**Last Updated**: 2026-03-17 | **Status**: Active

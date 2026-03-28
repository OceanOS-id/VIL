# Contributing to VIL

Thank you for your interest in contributing to VIL! This document provides guidelines for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/VIL.git`
3. Create a feature branch: `git checkout -b feat/your-feature`
4. Make your changes
5. Run checks: `cargo test && cargo clippy && cargo fmt --check`
6. Submit a pull request

## Development Setup

```bash
# Install Rust (pinned version via rust-toolchain.toml)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

## Code Standards

- **Formatting:** Run `cargo fmt` before committing
- **Linting:** All clippy warnings must be resolved
- **Tests:** Add tests for new functionality; maintain existing test coverage
- **Unsafe code:** Every `unsafe` block must have a `// SAFETY:` comment explaining why it is sound
- **Error handling:** Prefer `?` operator over `.unwrap()` in library code
- **Documentation:** Public items should have doc comments

## Pull Request Process

1. Update documentation if you changed public APIs
2. Add tests for new features or bug fixes
3. Ensure CI passes (check, test, clippy, fmt, audit, deny)
4. Keep PRs focused — one logical change per PR
5. Write a clear PR description explaining the "why"

## Commit Messages

Follow conventional commits:
- `feat:` new feature
- `fix:` bug fix
- `security:` security improvement
- `docs:` documentation only
- `refactor:` code change that neither fixes a bug nor adds a feature
- `test:` adding or updating tests
- `chore:` maintenance tasks

## Reporting Issues

- **Security vulnerabilities:** See [SECURITY.md](SECURITY.md) — do NOT open public issues
- **Bugs:** Open a GitHub issue with reproduction steps
- **Feature requests:** Open a GitHub issue with use case description

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.

# Reedy Development Guide

## Commands
- Build: `cargo build`
- Run: `cargo run`
- Check: `cargo check`
- Format: `cargo fmt`
- Lint: `cargo clippy`
- Test: `cargo test`
- Test specific: `cargo test test_name`
- Release build: `cargo build --release`

## Code Style Guidelines
- **Formatting**: Follow Rust standard formatting with `cargo fmt`
- **Imports**: Group imports logically (std first, external crates next, internal modules last)
- **Naming**: Use snake_case for variables/functions, PascalCase for types/traits
- **Error Handling**: Use Result<T, E> for functions that can fail, log errors with `log` crate
- **Documentation**: Document public APIs with doc comments (`///`)
- **Types**: Use strong typing and avoid `unwrap()` in production code
- **Modules**: Keep modules single-purpose and organized by functionality
- **Async**: Use `tokio` for async operations and properly handle async/await
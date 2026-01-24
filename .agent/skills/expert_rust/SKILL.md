---
name: Expert Rust Skill
description: Instructions, patterns, and best practices for writing idiomatic, high-performance, and safe Rust code.
---

# Expert Rust Skill

This skill serves as a guide for writing production-grade Rust code, focusing on safety, concurrency, and idiomatic patterns.

## 1. Ownership & Memory Management
- **Borrowing**: Prefer borrowing (`&T`, `&mut T`) over taking ownership when data doesn't need to be consumed.
- **Cloning**: Be explicit with `.clone()`. Avoid it in hot paths unless necessary. Use `Arc<T>` for shared ownership across threads/tasks.
- **Lifetimes**: Explicitly strictly define lifetimes `'a` only when the compiler cannot infer them.

## 2. Error Handling
- **Result & Option**: Never use `.unwrap()` or `.expect()` in production code unless you have proof that it will never panic (and comment that proof).
- **Propagation**: Use the `?` operator to propagate errors.
- **Crates**:
  - Use `thiserror` for library error definitions (enum implementation).
  - Use `anyhow` for application-level error handling and context (`.context("...")`).

## 3. Concurrency & Async
- **Runtime**: Use `tokio` for async operations.
- **Send & Sync**: Ensure types passed across thread/task boundaries implement `Send` (and `Sync` if shared).
- **Blocking**: NEVER block the async runtime thread. Use `tokio::task::spawn_blocking` for CPU-intensive or synchronous I/O operations.
- **Synchronization**: Use `tokio::sync::Mutex` for async critical sections, but prefer message passing (`tokio::sync::mpsc`) via channels over shared state where possible.

## 4. Type System & Traits
- **Newtypes**: Use the "Newtype" pattern (`struct ID(String)`) to enforce type safety and avoid primitives obsession.
- **Traits**: Prefer defining behavior via Traits. Use `impl Trait` for return types to reduce signature complexity.
- **Generics**: Use Generics with trait bounds (`T: AsRef<Path>`) to increase API flexibility.

## 5. Idiomatic Rust
- **Clippy**: Always respect `cargo clippy`. Treat warnings as errors.
- **Formatting**: Run `cargo fmt` automatically.
- **Iterators**: Prefer functional iterator chains (`.map()`, `.filter()`, `.collect()`) over raw `for` loops for readability and potential compiler optimizations.
- **Pattern Matching**: Use `match` with exhaustive patterns. Use `if let` for single pattern handling.

## 6. Testing
- **Unit Tests**: Place unit tests in the same file `mod tests { ... }`.
- **Integration Tests**: Place integration tests in `tests/` directory.
- **Doc Tests**: Write examples in documentation comments (`///`) which serve as tests.

## 7. Performance
- **Allocations**: Minimize heap allocations. Use `&str` instead of `String` when possible.
- **Data Structures**: Choose the right container (e.g., `HashMap`, `BTreeMap`, `Vec`) based on access patterns.
- **Buffering**: Use `BufReader`/`BufWriter` for I/O operations to reduce syscalls.

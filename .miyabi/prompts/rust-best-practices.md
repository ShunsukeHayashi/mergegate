# Rust Best Practices Checklist

When writing or reviewing Rust code, ensure:

## Error Handling
- [ ] Use `Result<T, E>` for fallible operations
- [ ] Avoid `.unwrap()` and `.expect()` in library code
- [ ] Use `?` operator for error propagation
- [ ] Create custom error types with `thiserror`
- [ ] Provide context with `.context()` or `.with_context()`

## Memory & Performance
- [ ] Prefer `&str` over `String` for function parameters
- [ ] Use `Cow<str>` when ownership is conditional
- [ ] Avoid unnecessary cloning
- [ ] Use iterators instead of index loops
- [ ] Consider `Vec::with_capacity()` for known sizes

## Safety
- [ ] Minimize `unsafe` code
- [ ] Document safety invariants for `unsafe` blocks
- [ ] Validate all external input
- [ ] Use `#[must_use]` for important return values

## Async
- [ ] Use `tokio::spawn` for concurrent tasks
- [ ] Avoid blocking operations in async context
- [ ] Use `tokio::select!` for racing futures
- [ ] Handle cancellation properly

## Testing
- [ ] Unit tests in same file with `#[cfg(test)]`
- [ ] Integration tests in `tests/` directory
- [ ] Use `#[should_panic]` for expected panics
- [ ] Test error cases, not just happy paths

## Documentation
- [ ] Doc comments for public items (`///`)
- [ ] Examples in doc comments
- [ ] Module-level documentation (`//!`)
- [ ] Link related items with `[`item`]`

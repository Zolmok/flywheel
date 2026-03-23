# CLAUDE.md

## Commands

- `cargo build` — build the project
- `cargo test` — run all tests
- `cargo fmt -- --check` — check formatting
- `cargo clippy` — lint

## Code Style

- Prefer functional style
- 4-space indentation, compatible with `cargo fmt`
- Always use braces `{}` for control structures (no braceless `if`/`else`/`for`/`while`)

## Error Handling

- Never use `unwrap()`, `expect()`, or `?`
- Use explicit `match` for all `Result` and `Option` handling

## Dead Code

- Do not use `#[allow(dead_code)]` — delete unused code instead

## Tests

- Tests live in a separate adjacent file: `foo.rs` → `foo_tests.rs`
- Include the source module with `#[path = "foo.rs"] mod foo;` or appropriate imports

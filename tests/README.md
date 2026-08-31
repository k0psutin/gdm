# Tests and development

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) and Cargo; the minimum supported version is defined by `Cargo.toml`
- The repository toolchain configuration in [`rust-toolchain.toml`](../rust-toolchain.toml)
- [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) for coverage
- [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb) for optional VS Code debugging

## Test structure

- Unit tests are colocated with the implementation in `src/`.
- Integration tests are in this directory and exercise the CLI commands.
- Shared fixtures and mock projects are in `tests/mocks`.
- Network-facing behavior is mocked in tests; running the test suite does not require access to the Godot Asset Library.

## Run the test suite

```bash
cargo test
```

Run one integration test file or a matching test by name when iterating:

```bash
cargo test --test list_command_tests
cargo test test_list_command
```

## Formatting and linting

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

## Coverage

```bash
cargo llvm-cov --lcov --output-path lcov.info
```

## CLI documentation checks

When changing a command or its output, compare the README with the generated help and run the relevant integration tests:

```bash
cargo run -- --help
cargo run -- list --help
cargo run -- add --help
```

The documentation recordings are maintained separately. See [`docs/README.md`](../docs/README.md) for the VHS workflow.

## Troubleshooting

- Ensure the active Rust toolchain satisfies the version in `Cargo.toml`.
- If a test fails after changing a fixture, inspect the related file under `tests/mocks` and the test's expected output.
- For platform-specific Rust issues, consult the [Rust documentation](https://doc.rust-lang.org/book/ch01-01-installation.html).

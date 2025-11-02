# Tests

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) (comes with Rust)
- [llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) for test coverage
- [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb) for debugging (VSCode extension)

## Test structuring

All services has unit tests under mod tests in the same file, except plugin service. Plugin service has a separate file `mod_tests.rs`.

Integration tests are inside `/tests` folder.

Unit tests uses `/tests/mocks` for some of their tests.

## Run all tests
```bash
cargo test
```

## Run tests with coverage
```bash
cargo llvm-cov --lcov --output-path lcov.info
```

## Troubleshooting

- Ensure Rust and Cargo are installed and up to date.
- For platform-specific issues, consult the [Rust documentation](https://doc.rust-lang.org/book/ch01-01-installation.html).

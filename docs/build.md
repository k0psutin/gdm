# Build Instructions for GDM

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) (comes with Rust)
- [llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) for test coverage

## Steps

1. **Clone the repository:**
    ```bash
    git clone https://github.com/k0psutin/gdm.git
    cd gdm
    ```

2. **Install tarpaulin**
    ```bash
    cargo install cargo-tarpaulin
    ```

2. **Build the project:**
    ```bash
    cargo build --release
    ```

3. **Run tests (optional):**
    ```bash
    cargo test
    ```

4. **Gather test coverage**
    ```bash
    cargo llvm-cov --lcov --output-path lcov.info
    ```

5. **Binary location:**
    - The compiled binary will be in `target/release/gdm`. You can also install the binary with `cargo install --path .` to compile binary to `.cargo/bin/gdm`


## Troubleshooting

- Ensure Rust and Cargo are installed and up to date.
- For platform-specific issues, consult the [Rust documentation](https://doc.rust-lang.org/book/ch01-01-installation.html).

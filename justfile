install:
  cargo install cargo-audit --locked
  cargo install cargo-machete

check:
  cargo fmt
  cargo clippy
  cargo audit --deny warnings
  cargo machete
  cargo check
  cargo test

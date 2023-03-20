run-dev:
    RUST_LOG=WARN,contact_group_auth=DEBUG RUST_BACKTRACE=full cargo r
run-r:
    cargo build -r
    RUST_LOG=WARN,contact_group_auth=DEBUG ./target/release/contact-group-auth
check:
    cargo fmt --check --all
    cargo clippy --all
test:
    cargo test
fix: 
    cargo fmt
    cargo clippy --fix --allow-staged

commit:
    cargo fmt --check --all
    cargo clippy --all
    git commit
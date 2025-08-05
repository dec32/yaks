import "justfile.local"

fmt *ARGS:
    cargo +nightly fmt --all
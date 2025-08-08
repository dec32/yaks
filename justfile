import "justfile.local"

fmt *ARGS:
    cargo +nightly fmt --all
gui *ARGS:
    cargo build --package yaks-gui
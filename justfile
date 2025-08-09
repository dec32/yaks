fmt *ARGS:
    cargo +nightly fmt --all
gui *ARGS:
    cargo run --package yaks-gui
test:
    cargo run --package yaks-tui -- patreon/470718

fmt *ARGS:
    cargo +nightly fmt --all
gui *ARGS:
    cargo run --package yaks-gui
test:
    cargo run --package yaks-tui -- patreon/470718 --range 67928516..

fmt:
    cargo +nightly fmt --all
tui *args:
    cargo run --package yaks-tui -- {{args}}
gui *args:
    cargo run --package yaks-gui -- {{args}}
test *args:
    just tui patreon/470718 --range 67928516.. --text {{args}}
install:
    cargo install --path ./yaks-tui
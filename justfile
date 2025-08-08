import "justfile.local"

fmt *ARGS:
    cargo +nightly fmt --all
view *ARGS:
    slint-viewer yaks-gui/ui/ui.slint --auto-reload
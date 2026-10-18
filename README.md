# Sandtris

A falling-sand puzzle game in Rust ([macroquad](https://macroquad.rs)). Connect same-coloured sand from the left wall to the right wall to clear it.

## Controls

| Key | Action |
| --- | --- |
| A / D or ←/→ | Aim |
| W or ↑ | Rotate |
| Space or click | Drop |
| P | Pause |
| M | Menu (volume) |
| Enter | Restart after game over |

## Run natively

Run from the project root (Linux needs `libasound2-dev`):

    cargo run --release

## Web build

    rustup target add wasm32-unknown-unknown
    cargo build --release --target wasm32-unknown-unknown

Pushing to `main` deploys `web/index.html`, the wasm binary and `assets/music/*.ogg` to GitHub Pages via `.github/workflows/pages.yml`.

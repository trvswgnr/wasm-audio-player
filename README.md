# WASM Audio Player

This is a simple audio player written in Rust and compiled to WASM with [wasm-pack](https://github.com/rustwasm/wasm-pack) and [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen).

It demonstrates how to use the [Web Audio API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Audio_API) from Rust and interact with Rust from JavaScript.

## Build

```bash
cargo install wasm-pack # if you don't have it already
git clone https://github.com/trvswgnr/wasm-audio-player.git
cd wasm-audio-player
wasm-pack build --release --target web
```

## Test

```bash
wasm-pack test --headless --chrome --firefox
```

## See it in action

After building, simply open `index.html` (in the root directory) in your browser.


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

## Performance Observations

Overall the WASM version is currently slower than the vanilla JS version. For the first round, the initial play is around **16.4%** slower, the pause times for both are about the same, but the resumes are around **197%** slower. On average, the WASM version is **~59%** slower than the JS version.

The WASM binary is currently **700%** larger than the JS file (32KB vs 4KB); the difference would be even more dramatic if the JS file was minified.

Here are some reasons I think the WASM version is slower:
- The WASM version is currently using a single thread, so it's not taking advantage of the multi-core CPUs that most computers have.
- Binding to the Web Audio API is currently slower than using the JS API directly, since it's using a lot of `JsValue` conversions.
- From `wasm-bindgen`: "`JsValue` doesn't actually live in Rust right now but actually in a table owned by the wasm-bindgen generated JS glue code." This means that the WASM version is doing a lot more work to convert the `JsValue` to a Rust type. This is probably the biggest reason the WASM version is slower, but in the future, this should be fixed.
- This is a simple example that doesn't do much, so the overhead of the WASM version is likely more noticeable than it would be if it was performing more complex tasks.
- I'm still learning Rust, so my code could almost certainly be more efficient.

### Results

|    Run    |          WASM        |         JS          |
| --------- | -------------------- | ------------------- |
|1 (play)   | 53.39999997615814ms  | 45.89999997615814ms |
|2 (pause)  | 5.299999952316284ms  | 6.299999952316284ms |
|3 (resume) | 19.799999952316284ms | 6.5ms               |
|4 (pause)  | 5.599999904632568ms  | 6.800000071525574ms |
|5 (resume) | 14.799999952316284ms | 5.899999976158142ms |
|6 (pause)  | 6.700000047683716ms  | 5.099999904632568ms |
|7 (resume) | 20.899999976158142ms | 6.300000071525574ms |
|8 (pause)  | 6.199999928474426ms  | 6.699999928474426ms |
|9 (resume) | 19.800000071525574ms | 5.399999976158142ms |
|10 (pause) | 6.299999952316284ms  | 5.100000023841858ms |
|           |                      |                     |
| Average   | ~15.9ms              | ~10ms               |

## License

[MIT](LICENSE)

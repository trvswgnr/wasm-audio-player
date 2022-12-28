use js_sys::{Array, Uint8Array};
use std::{cell::RefCell, future::Future, rc::Rc};
use wasm_audio_player::{await_js, play_file, rc_refcell, set_timeout, use_rc};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;
use web_sys::{AudioContext, AudioContextState, GainNode};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_play_file() {
    let file_bytes = include_bytes!("test.wav");
    let file_array = Uint8Array::from(file_bytes.as_ref());
    let rc_file_array = rc_refcell!(file_array.clone());

    // test playing a file initially
    let initial_play = run_play_test(
        rc_file_array.clone(),
        None,
        None,
        AudioContextState::Running,
        1.0,
    )
    .await;
    assert!(initial_play.is_ok(), "{:?}", initial_play);

    // test pausing a file
    let (context, gain_node) = initial_play.unwrap();
    let second_play = run_play_test(
        rc_file_array.clone(),
        Some(context),
        Some(gain_node),
        AudioContextState::Suspended,
        0.0,
    )
    .await;
    assert!(second_play.is_ok(), "{:?}", second_play);

    // test resuming a file
    let (context, gain_node) = second_play.unwrap();
    let third_play = run_play_test(
        rc_file_array.clone(),
        Some(context),
        Some(gain_node),
        AudioContextState::Running,
        1.0,
    )
    .await;
    assert!(third_play.is_ok(), "{:?}", third_play);

    // close the audio context
    let (context, _) = third_play.unwrap();
    await_js!(context.close().unwrap()).unwrap();

    // make sure the context is closed
    assert_eq!(context.state(), AudioContextState::Closed);
}

/// Runs a test for playing a file.
async fn run_play_test(
    rc_file_array: Rc<RefCell<Uint8Array>>,
    context: Option<AudioContext>,
    gain_node: Option<GainNode>,
    expected_state: AudioContextState,
    expected_gain: f32,
) -> Result<(AudioContext, GainNode), JsValue> {
    let play = play_file(use_rc!(rc_file_array), context, gain_node).await;

    assert!(play.is_ok(), "{:?}", play);

    sleep(500).await;

    let result = play?.to_owned().dyn_into::<Array>()?;
    let context = result.get(0).dyn_into::<AudioContext>()?;
    let gain_node = result.get(1).dyn_into::<GainNode>()?;

    assert_eq!(context.state(), expected_state);
    assert_eq!(gain_node.gain().value(), expected_gain);

    Ok((context, gain_node))
}

/// Sleeps for a duration.
fn sleep(timeout: u32) -> impl Future<Output = ()> {
    let (sender, receiver) = oneshot::channel();
    set_timeout(
        move || {
            sender.send(()).unwrap();
            Ok(())
        },
        timeout,
    );
    // convert receiver to a future
    async move {
        receiver.await.unwrap();
    }
}

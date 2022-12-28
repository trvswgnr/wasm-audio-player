#![feature(async_closure)] // allow async closure syntax
#![feature(once_cell)] // allow OnceCell
#![allow(non_upper_case_globals)] // fix for wasm-bindgen generated code

use std::{cell::RefCell, future::Future, rc::Rc};

use js_sys::Uint8Array;
use wasm_bindgen::{prelude::*, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, AudioBuffer, AudioContext, AudioContextState, GainNode};

/// Plays an audio file.
///
/// The user will upload a file with an HTML input element, and we'll pass the
/// file to this function from JavaScript.
///
/// # JavaScript Usage
/// ```js
/// import init, { play_file } from './pkg/wasm_audio_player.js';
/// let audioContext = null;
/// let gainNode = null;
/// const file = document.getElementById("audioFile").files[0];
/// const fileData = await file.arrayBuffer();
/// const bytes = new Uint8Array(fileData);
/// play_file(bytes, audioContext, gainNode).then(data => {
///     audioContext = data[0];
///     gainNode = data[1];
/// });
#[wasm_bindgen]
pub async fn play_file(
    file_array: Uint8Array,
    context: Option<AudioContext>,
    gain_node: Option<GainNode>,
) -> Result<JsValue, JsValue> {
    let mut using_existing_context = false;
    let taper = 0.2;
    let audio_ctx: AudioContext = match context {
        Some(ctx) => {
            console::log_1(&"using existing audio context".into());
            using_existing_context = true;
            ctx
        }
        None => {
            console::log_1(&"creating new audio context".into());
            AudioContext::new()?
        }
    };

    let mut return_data = ReturnData::new().unwrap();

    if using_existing_context {
        if gain_node.is_none() {
            console::log_1(&"no gain node".into());
            return Err("no gain node".into());
        }

        // if the audio context is closed, we can't do anything with it
        if audio_ctx.state() == AudioContextState::Closed {
            console::log_1(&"audio context is closed".into());
            return_data.set_audio_ctx(Some(audio_ctx));
            return_data.set_gain_node(gain_node);
            return Ok(return_data.into());
        }

        // if already playing, pause
        if audio_ctx.state() == AudioContextState::Running {
            // taper the volume down to 0 to avoid a click
            gain_node
                .as_ref()
                .unwrap()
                .gain()
                .linear_ramp_to_value_at_time(0.0, audio_ctx.current_time() + taper)?;

            let rc_audio_ctx = rc_refcell!(audio_ctx.clone());

            set_timeout_async(
                async move {
                    // stop the audio
                    let ctx = rc_audio_ctx.as_ref().borrow();
                    await_js!(ctx.suspend()?)?;
                    console::log_1(&"state should be stopped".into());
                    console::log_1(&format!("state: {:?}", ctx.state()).into());
                    Ok(())
                },
                (taper * 1000.0) as u32,
            );

            console::log_1(&"paused".into());
            return_data.set_audio_ctx(Some(audio_ctx));
            return_data.set_gain_node(gain_node);
            return Ok(return_data.into());
        }

        // if paused, resume
        if audio_ctx.state() == AudioContextState::Suspended {
            // taper the volume up to 1
            gain_node
                .as_ref()
                .unwrap()
                .gain()
                .linear_ramp_to_value_at_time(1.0, audio_ctx.current_time() + taper)?;

            let rc_audio_ctx = rc_refcell!(audio_ctx.clone());
            let ctx = rc_audio_ctx.as_ref().borrow();

            await_js!(ctx.resume()?)?;

            console::log_1(&"state should be running".into());
            console::log_1(&format!("state: {:?}", ctx.state()).into());
            return_data.set_audio_ctx(Some(audio_ctx));
            return_data.set_gain_node(gain_node);
            return Ok(return_data.into());
        }
    }

    let decoded = audio_ctx.decode_audio_data(&file_array.buffer())?;

    // wait for the audio to be decoded
    let decoded = JsFuture::from(decoded).await?;

    // play the audio
    let decoded = decoded.dyn_into::<AudioBuffer>()?;
    let source = audio_ctx.create_buffer_source()?;
    source.set_buffer(Some(&decoded));
    let gain_node = audio_ctx.create_gain()?;
    gain_node.gain().set_value(1.0);
    gain_node.connect_with_audio_node(&audio_ctx.destination())?;
    source.connect_with_audio_node(&gain_node)?;
    source.start()?;

    console::log_1(&"state should be running".into());
    console::log_1(&format!("state: {:?}", audio_ctx.state()).into());

    return_data.set_audio_ctx(Some(audio_ctx));
    return_data.set_gain_node(Some(gain_node));

    Ok(return_data.into())
}

/// A wrapper for window.setTimeout that takes a closure instead of a function.
///
/// # Example
/// ```ignore
/// set_timeout(|| {
///    /* do something */
/// }, 1000);
/// ```
#[allow(dead_code)]
pub fn set_timeout<F>(f: F, timeout: u32)
where
    F: FnOnce() -> Result<(), JsValue> + 'static,
{
    let f = Closure::once_into_js(move || {
        f().expect("failed to run closure");
    });

    web_sys::window()
        .unwrap()
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            f.as_ref().unchecked_ref(),
            timeout as i32,
        )
        .unwrap();
}

/// A version of `set_timeout` that takes a future instead of a closure.
///
/// This is useful for calling async functions from JavaScript.
/// The future will be run on the current thread.
/// The future will be run in a `spawn_local` call, so it will not block the
/// current thread.
///
/// This is useful for calling async functions from JavaScript.
///
/// ```ignore
/// set_timeout_async(async move {
///    // do something async
/// }, 1000);
/// ```
///
/// This is equivalent to:
///
/// ```ignore
/// set_timeout(async move {
///    wasm_bindgen_futures::spawn_local(async move {
///       /* do something async */
///   });
/// }, 1000);
/// ```
fn set_timeout_async<F>(f: F, timeout: u32)
where
    F: Future<Output = Result<(), JsValue>> + 'static,
{
    set_timeout(
        || {
            wasm_bindgen_futures::spawn_local(async move {
                f.await.expect("failed to run future");
                ()
            });
            Ok(())
        },
        timeout,
    );
}

/// A macro to create an `Rc<RefCell<T>>` from a value.
///
/// This is useful for passing a value to a closure that needs to be moved into
/// the closure, but we want to be able to access the value after the closure
/// has been called.
///
/// For example, we can use this to pass a `AudioContext` to a closure that
/// needs to be moved into the closure, but we want to be able to access the
/// `AudioContext` after the closure has been called.
///
/// ```ignore
/// let rc_audio_ctx = rc_refcell!(audio_ctx.clone());
/// set_timeout_async(
///     async move {
///         await_js!(rc_audio_ctx.borrow().suspend()?)?;
///         console::log_1(&"stopped".into());
///         Ok(())
///     },
///     1000,
/// );
/// ```
#[macro_export]
macro_rules! rc_refcell {
    ($value:expr) => {
        Rc::new(RefCell::new($value))
    };
}

#[macro_export]
macro_rules! use_rc {
    ($value:expr) => {
        $value.as_ref().borrow().clone()
    };
}

/// A macro to await a JavaScript future.
///
/// This is useful for calling async functions from JavaScript.
///
/// ```ignore
/// set_timeout_async(async move {
///     await_js!(rc_audio_ctx.borrow().suspend()?)?;
///     console::log_1(&"stopped".into());
///     Ok(())
/// }, 1000);
/// ```
///
/// ```ignore
/// set_timeout_async(async move {
///    await_js! {
///        rc_audio_ctx.borrow().suspend()?;
///    }
/// }, 1000);
/// ```
#[macro_export]
macro_rules! await_js {
    ($value:expr) => {
        wasm_bindgen_futures::JsFuture::from($value).await
    };
}

#[wasm_bindgen]
pub struct ReturnData {
    audio_ctx: Option<AudioContext>,
    gain_node: Option<GainNode>,
}

#[wasm_bindgen]
impl ReturnData {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<ReturnData, JsValue> {
        let audio_ctx = None;
        let gain_node = None;
        Ok(ReturnData {
            audio_ctx,
            gain_node,
        })
    }

    pub fn set_audio_ctx(&mut self, audio_ctx: Option<AudioContext>) {
        self.audio_ctx = audio_ctx.clone();
    }

    pub fn set_gain_node(&mut self, gain_node: Option<GainNode>) {
        self.gain_node = gain_node.clone();
    }

    #[wasm_bindgen(getter)]
    #[wasm_bindgen(js_name = audioCtx)]
    pub fn get_audio_ctx(&self) -> Option<AudioContext> {
        self.audio_ctx.clone()
    }

    #[wasm_bindgen(getter)]
    #[wasm_bindgen(js_name = gainNode)]
    pub fn get_gain_node(&self) -> Option<GainNode> {
        self.gain_node.clone()
    }
}

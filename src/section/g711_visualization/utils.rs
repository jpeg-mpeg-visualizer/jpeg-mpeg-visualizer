use web_sys::{AudioContext, AudioBuffer, AudioContextOptions};
use seed::JsFuture;
use wasm_bindgen::JsCast;
use seed::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

fn convert32f_to_16i(original: Vec<f32>) -> Vec<i16> {
    original
        .iter()
        .map(|sample| {
            let value = (sample * i16::MAX as f32).floor();
            value.clamp(i16::MIN as f32, i16::MAX as f32) as i16
        }).collect::<Vec<i16>>()
}

pub(super) async fn load_audio(file_blob: gloo_file::Blob) -> (Vec<i16>, f32, u32, Vec<i16>, u32) {
    let array_buffer = gloo_file::futures::read_as_array_buffer(&file_blob)
        .await
        .unwrap();

    let temp_array_buffer = gloo_file::futures::read_as_array_buffer(&file_blob)
        .await
        .unwrap();

    let context: AudioContext = AudioContext::new().unwrap();
    let buffer_original: AudioBuffer = JsFuture::from(
        context.decode_audio_data(&array_buffer).unwrap()
    ).await.unwrap()
        .dyn_into::<AudioBuffer>().unwrap();

    let sample16i_original = convert32f_to_16i(buffer_original.get_channel_data(0).unwrap());

    let mut options_8khz = AudioContextOptions::new();
    options_8khz.sample_rate(8000_f32);
    let context_8khz = AudioContext::new_with_context_options(&options_8khz).unwrap();
    let buffer_8khz = JsFuture::from(
        context_8khz.decode_audio_data(&temp_array_buffer).unwrap()
    ).await.unwrap()
        .dyn_into::<AudioBuffer>().unwrap();

    let sample16i_8khz = convert32f_to_16i(buffer_8khz.get_channel_data(0).unwrap());

    (sample16i_original, buffer_original.sample_rate(), buffer_original.length(), sample16i_8khz, buffer_8khz.length())
}

use std::ops::Deref;
use std::rc::Rc;

use seed::JsFuture;
use seed::{prelude::wasm_bindgen, wasm_bindgen_futures};
use wasm_bindgen::JsCast;
use web_sys::{AudioBuffer, AudioContext, AudioContextOptions, OfflineAudioContext};

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
        })
        .collect::<Vec<i16>>()
}

pub(super) async fn load_audio(file_blob: gloo_file::Blob) -> (Vec<i16>, f32, u32, Vec<i16>, u32) {
    let array_buffer = gloo_file::futures::read_as_array_buffer(&file_blob)
        .await
        .unwrap();

    let temp_array_buffer = gloo_file::futures::read_as_array_buffer(&file_blob)
        .await
        .unwrap();

    let mut options_44khz = AudioContextOptions::new();
    options_44khz.sample_rate(44100_f32);

    let context: AudioContext = AudioContext::new_with_context_options(&options_44khz).unwrap();
    let buffer_original: AudioBuffer =
        JsFuture::from(context.decode_audio_data(&array_buffer).unwrap())
            .await
            .unwrap()
            .dyn_into::<AudioBuffer>()
            .unwrap();

    let sample16i_original = convert32f_to_16i(buffer_original.get_channel_data(0).unwrap());

    let mut options_8khz = AudioContextOptions::new();
    options_8khz.sample_rate(8000_f32);
    let context_8khz = AudioContext::new_with_context_options(&options_8khz).unwrap();
    let buffer_8khz = JsFuture::from(context_8khz.decode_audio_data(&temp_array_buffer).unwrap())
        .await
        .unwrap()
        .dyn_into::<AudioBuffer>()
        .unwrap();

    let sample16i_8khz = convert32f_to_16i(buffer_8khz.get_channel_data(0).unwrap());

    (
        sample16i_original,
        buffer_original.sample_rate(),
        buffer_original.length(),
        sample16i_8khz,
        buffer_8khz.length(),
    )
}

pub async fn get_upsampled_pcm(
    buffer: Rc<Option<AudioBuffer>>,
    target_sample_rate: f32,
) -> Vec<i16> {
    let offline_context =
        OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(
            1,
            (buffer.deref().as_ref().unwrap().duration() * target_sample_rate as f64) as u32,
            target_sample_rate,
        )
        .unwrap();
    let source = offline_context.create_buffer_source().unwrap();
    source.set_buffer(buffer.deref().as_ref());
    source
        .connect_with_audio_node(&offline_context.destination())
        .unwrap();
    source.start().unwrap();
    let resampled = offline_context.start_rendering().unwrap();
    let resampled = wasm_bindgen_futures::JsFuture::from(resampled)
        .await
        .unwrap();
    let upsampled_buffer = resampled.dyn_into::<AudioBuffer>().unwrap();
    let channel_data = upsampled_buffer.get_channel_data(0).unwrap();
    convert32f_to_16i(channel_data)
}

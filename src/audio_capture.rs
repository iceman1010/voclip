use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use tokio::sync::mpsc;

use crate::error::VoclipError;

pub struct AudioCapture {
    #[allow(dead_code)]
    stream: Stream,
    pub device_sample_rate: u32,
    #[allow(dead_code)]
    channels: u16,
}

pub fn start_capture(tx: mpsc::Sender<Vec<i16>>) -> Result<AudioCapture, VoclipError> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| VoclipError::AudioDevice("No input device found".into()))?;

    let config = device
        .default_input_config()
        .map_err(|e| VoclipError::AudioDevice(e.to_string()))?;

    let device_sample_rate = config.sample_rate().0;
    let channels = config.channels();
    let sample_format = config.sample_format();

    let err_fn = |e: cpal::StreamError| {
        eprintln!("Audio stream error: {e}");
    };

    let stream = match sample_format {
        SampleFormat::F32 => device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mono = to_mono_i16_from_f32(data, channels);
                    let _ = tx.try_send(mono);
                },
                err_fn,
                None,
            )
            .map_err(|e| VoclipError::AudioDevice(e.to_string()))?,
        SampleFormat::I16 => device
            .build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let mono = to_mono_i16(data, channels);
                    let _ = tx.try_send(mono);
                },
                err_fn,
                None,
            )
            .map_err(|e| VoclipError::AudioDevice(e.to_string()))?,
        _ => {
            return Err(VoclipError::AudioDevice(format!(
                "Unsupported sample format: {sample_format}"
            )));
        }
    };

    stream
        .play()
        .map_err(|e| VoclipError::AudioDevice(e.to_string()))?;

    Ok(AudioCapture {
        stream,
        device_sample_rate,
        channels,
    })
}

fn to_mono_i16_from_f32(data: &[f32], channels: u16) -> Vec<i16> {
    let ch = channels as usize;
    data.chunks(ch)
        .map(|frame| {
            let sum: f32 = frame.iter().sum();
            let avg = sum / ch as f32;
            (avg * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16
        })
        .collect()
}

fn to_mono_i16(data: &[i16], channels: u16) -> Vec<i16> {
    let ch = channels as usize;
    if ch == 1 {
        return data.to_vec();
    }
    data.chunks(ch)
        .map(|frame| {
            let sum: i32 = frame.iter().map(|&s| s as i32).sum();
            (sum / ch as i32) as i16
        })
        .collect()
}

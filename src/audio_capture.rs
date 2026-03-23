use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use tokio::sync::mpsc;

use crate::error::VoclipError;

/// Temporarily suppress stderr (ALSA/Jack spam) during a closure.
#[cfg(unix)]
fn with_stderr_suppressed<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    use std::fs::File;
    use std::os::fd::AsRawFd;

    // Open /dev/null and save the current stderr fd
    let Ok(devnull) = File::open("/dev/null") else {
        return f();
    };
    let stderr_fd = 2;
    let saved = unsafe { libc::dup(stderr_fd) };
    if saved < 0 {
        return f();
    }

    // Redirect stderr to /dev/null
    unsafe { libc::dup2(devnull.as_raw_fd(), stderr_fd) };
    let result = f();

    // Restore stderr
    unsafe { libc::dup2(saved, stderr_fd) };
    unsafe { libc::close(saved) };
    result
}

#[cfg(not(unix))]
fn with_stderr_suppressed<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    f()
}

pub struct AudioCapture {
    #[allow(dead_code)]
    stream: Stream,
    pub device_sample_rate: u32,
    #[allow(dead_code)]
    channels: u16,
}

/// List all available input devices (suppresses ALSA/Jack noise).
pub fn list_input_devices() -> Result<Vec<String>, VoclipError> {
    with_stderr_suppressed(|| {
        let host = cpal::default_host();
        let devices = host
            .input_devices()
            .map_err(|e| VoclipError::AudioDevice(e.to_string()))?;
        let mut names = Vec::new();
        for device in devices {
            if let Ok(name) = device.name() {
                names.push(name);
            }
        }
        Ok(names)
    })
}

/// Find an input device by name (case-insensitive substring match).
fn find_device_by_name(name: &str) -> Result<cpal::Device, VoclipError> {
    with_stderr_suppressed(|| {
        let host = cpal::default_host();
        let devices = host
            .input_devices()
            .map_err(|e| VoclipError::AudioDevice(e.to_string()))?;
        let lower = name.to_lowercase();
        for device in devices {
            if let Ok(dev_name) = device.name()
                && dev_name.to_lowercase().contains(&lower)
            {
                return Ok(device);
            }
        }
        Err(VoclipError::AudioDevice(format!(
            "No input device matching \"{name}\". Use --list-devices to see available devices."
        )))
    })
}

pub fn start_capture_with_device(
    tx: mpsc::Sender<Vec<i16>>,
    device_name: Option<&str>,
) -> Result<AudioCapture, VoclipError> {
    let host = cpal::default_host();
    let device = if let Some(name) = device_name {
        find_device_by_name(name)?
    } else {
        host.default_input_device()
            .ok_or_else(|| VoclipError::AudioDevice("No input device found".into()))?
    };

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

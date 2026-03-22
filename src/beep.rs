use std::f32::consts::PI;
use std::io::Cursor;
use std::time::Duration;

use crate::error::VoclipError;

/// Rising two-tone chirp: 400Hz → 800Hz (recording started)
pub fn play_start_beep() -> Result<(), VoclipError> {
    let tones = &[(400.0, 150), (800.0, 150)];
    play_tones(tones)
}

/// Falling two-tone chirp: 800Hz → 400Hz (recording done)
pub fn play_stop_beep() -> Result<(), VoclipError> {
    let tones = &[(800.0, 150), (400.0, 150)];
    play_tones(tones)
}

/// Sad descending "womp womp" tone for errors
pub fn play_error_beep() -> Result<(), VoclipError> {
    let tones = &[(400.0, 200), (200.0, 200), (350.0, 200), (175.0, 200)];
    play_tones(tones)
}

fn play_tones(tones: &[(f32, u32)]) -> Result<(), VoclipError> {
    let sample_rate = 44100u32;
    let fade_samples = (sample_rate as f32 * 0.005) as usize; // 5ms fade

    let mut all_samples: Vec<i16> = Vec::new();

    for &(freq, duration_ms) in tones {
        let num_samples = (sample_rate * duration_ms / 1000) as usize;
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let mut sample = (2.0 * PI * freq * t).sin() * 0.8;

            if i < fade_samples {
                sample *= i as f32 / fade_samples as f32;
            }
            if i >= num_samples - fade_samples {
                sample *= (num_samples - i) as f32 / fade_samples as f32;
            }

            all_samples.push((sample * i16::MAX as f32) as i16);
        }
    }

    let num_samples = all_samples.len();
    let data_size = (num_samples * 2) as u32;
    let file_size = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + data_size as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    for s in &all_samples {
        wav.extend_from_slice(&s.to_le_bytes());
    }

    let (_stream, handle) =
        rodio::OutputStream::try_default().map_err(|e| VoclipError::Playback(e.to_string()))?;
    let source =
        rodio::Decoder::new(Cursor::new(wav)).map_err(|e| VoclipError::Playback(e.to_string()))?;
    let sink = rodio::Sink::try_new(&handle).map_err(|e| VoclipError::Playback(e.to_string()))?;
    sink.append(source);
    sink.sleep_until_end();
    // Give the audio backend time to flush its buffer before dropping the stream
    std::thread::sleep(Duration::from_millis(50));

    Ok(())
}

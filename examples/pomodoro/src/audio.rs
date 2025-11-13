use std::io::Cursor;

use rodio::{Decoder, OutputStreamBuilder};

pub fn play_notification_sound(volume: f32) {
  // Spawn a thread to play the sound so it doesn't block the UI
  std::thread::spawn(move || {
    // Simple beep sound - a sine wave at 440Hz (A4 note)
    const SAMPLE_RATE: u32 = 44100;
    const FREQUENCY: f32 = 440.0;
    const DURATION_SECS: f32 = 0.5;

    let samples: Vec<i16> = (0..(SAMPLE_RATE as f32 * DURATION_SECS) as usize)
      .map(|i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        ((t * FREQUENCY * 2.0 * std::f32::consts::PI).sin() * (i16::MAX as f32 * 0.5)) as i16
      })
      .collect();

    // Create WAV file in memory
    let mut wav_data = Vec::new();
    // WAV header
    wav_data.extend_from_slice(b"RIFF");
    let file_size = 36 + (samples.len() * 2) as u32;
    wav_data.extend_from_slice(&file_size.to_le_bytes());
    wav_data.extend_from_slice(b"WAVE");

    // fmt chunk
    wav_data.extend_from_slice(b"fmt ");
    wav_data.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    wav_data.extend_from_slice(&1u16.to_le_bytes()); // audio format (PCM)
    wav_data.extend_from_slice(&1u16.to_le_bytes()); // num channels
    wav_data.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav_data.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // byte rate
    wav_data.extend_from_slice(&2u16.to_le_bytes()); // block align (num channels * bits per sample / 8)
    wav_data.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    // data chunk
    wav_data.extend_from_slice(b"data");
    wav_data.extend_from_slice(&((samples.len() * 2) as u32).to_le_bytes());
    for sample in samples {
      wav_data.extend_from_slice(&sample.to_le_bytes());
    }

    let mut stream_handle =
      OutputStreamBuilder::open_default_stream().expect("open default audio stream");
    stream_handle.log_on_drop(false);
    let sink = rodio::Sink::connect_new(stream_handle.mixer());
    sink.set_volume(volume);
    let cursor = Cursor::new(wav_data);
    match Decoder::new(cursor) {
      Ok(source) => {
        sink.append(source);
        sink.sleep_until_end();
      }
      Err(e) => {
        println!("Failed to decode audio: {}", e);
      }
    }
    sink.stop();
  });
}

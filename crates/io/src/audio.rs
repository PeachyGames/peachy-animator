//! A bounded WAV snapshot and waveform envelope. No audio device or editor state.
use std::{fs::File, io::Read, path::Path};

pub const MAX_WAV_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_WAVE_BINS: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WavePeak {
    pub min: f32,
    pub max: f32,
}

pub struct WavClip {
    /// The exact validated source snapshot, also used by the native audio decoder.
    pub bytes: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub peaks: Vec<WavePeak>,
}

impl WavClip {
    pub fn duration(&self) -> f64 {
        self.frames as f64 / f64::from(self.sample_rate)
    }
}

/// Read once, with a hard bound even if the file grows after metadata inspection.
pub fn load_wav(path: &Path) -> Result<WavClip, String> {
    let file = File::open(path).map_err(|error| format!("Cannot open WAV: {error}"))?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("Choose a regular WAV file".into());
    }
    if metadata.len() > MAX_WAV_BYTES as u64 {
        return Err("WAV files are limited to 64 MiB".into());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_WAV_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot read WAV: {error}"))?;
    decode_wav(bytes)
}

/// Decode only little-endian RIFF PCM8/16/24/32 or finite IEEE float32,
/// mono/stereo at 8–192 kHz. Unsupported codecs are rejected, never approximated.
pub fn decode_wav(bytes: Vec<u8>) -> Result<WavClip, String> {
    if bytes.len() > MAX_WAV_BYTES {
        return Err("WAV files are limited to 64 MiB".into());
    }
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("Expected a little-endian RIFF/WAVE file".into());
    }
    let end = (read_u32(&bytes[4..8]) as usize)
        .checked_add(8)
        .ok_or("Invalid WAV container length")?;
    if end != bytes.len() {
        return Err("WAV container length does not match its source bytes".into());
    }
    let mut format = None;
    let mut data = None;
    let mut offset = 12usize;
    while offset < end {
        let start = offset.checked_add(8).ok_or("Invalid WAV chunk length")?;
        if start > end {
            return Err("Truncated WAV chunk header".into());
        }
        let length = read_u32(&bytes[offset + 4..start]) as usize;
        let stop = start
            .checked_add(length)
            .ok_or("Invalid WAV chunk length")?;
        let padded = stop
            .checked_add(length & 1)
            .ok_or("Invalid WAV chunk padding")?;
        if padded > end {
            return Err("Truncated WAV chunk or missing odd-byte padding".into());
        }
        match &bytes[offset..offset + 4] {
            b"fmt " => {
                if format.replace(start..stop).is_some() {
                    return Err("Multiple WAV format chunks are not supported".into());
                }
            }
            b"data" => {
                if data.replace(start..stop).is_some() {
                    return Err("Multiple WAV data chunks are not supported".into());
                }
            }
            _ => {} // Metadata remains unchanged in the retained byte snapshot.
        }
        offset = padded;
    }
    let header = &bytes[format.ok_or("WAV has no format chunk")?];
    if header.len() < 16 || header.len() == 17 {
        return Err("Truncated WAV format chunk".into());
    }
    if header.len() >= 18 && usize::from(read_u16(&header[16..18])) > header.len() - 18 {
        return Err("Truncated WAV format extension".into());
    }
    let codec = read_u16(&header[..2]);
    let channels = read_u16(&header[2..4]);
    let sample_rate = read_u32(&header[4..8]);
    let byte_rate = read_u32(&header[8..12]);
    let alignment = read_u16(&header[12..14]);
    let bits = read_u16(&header[14..16]);
    if !matches!(channels, 1 | 2) {
        return Err("WAV audition supports mono or stereo audio".into());
    }
    if !(8_000..=192_000).contains(&sample_rate) {
        return Err("WAV sample rate must be between 8 and 192 kHz".into());
    }
    if !(codec == 1 && matches!(bits, 8 | 16 | 24 | 32) || codec == 3 && bits == 32) {
        return Err("WAV codec is unsupported; use PCM8/16/24/32 or IEEE float32".into());
    }
    let sample_bytes = usize::from(bits / 8);
    let frame_bytes = usize::from(channels) * sample_bytes;
    if usize::from(alignment) != frame_bytes
        || sample_rate.checked_mul(u32::from(alignment)) != Some(byte_rate)
    {
        return Err("WAV block alignment or byte rate is inconsistent".into());
    }
    let samples = &bytes[data.ok_or("WAV has no data chunk")?];
    if samples.is_empty() || !samples.len().is_multiple_of(frame_bytes) {
        return Err("WAV data must contain complete nonempty sample frames".into());
    }
    let frames = (samples.len() / frame_bytes) as u64;
    let bins = frames.min(MAX_WAVE_BINS as u64) as usize;
    let mut peaks = vec![
        WavePeak {
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
        };
        bins
    ];
    for (frame, values) in samples.chunks_exact(frame_bytes).enumerate() {
        let bin = ((frame as u64 * bins as u64) / frames) as usize;
        for value in values.chunks_exact(sample_bytes) {
            let value = sample(value, codec, bits);
            if !value.is_finite() {
                return Err("WAV float samples must be finite".into());
            }
            peaks[bin].min = peaks[bin].min.min(value);
            peaks[bin].max = peaks[bin].max.max(value);
        }
    }
    Ok(WavClip {
        bytes,
        sample_rate,
        channels,
        frames,
        peaks,
    })
}

fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}
fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}
fn sample(bytes: &[u8], codec: u16, bits: u16) -> f32 {
    if codec == 3 {
        return f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    }
    match bits {
        8 => (f32::from(bytes[0]) - 128.) / 128.,
        16 => f32::from(i16::from_le_bytes([bytes[0], bytes[1]])) / 32768.,
        24 => {
            let value =
                ((i32::from(bytes[0]) | i32::from(bytes[1]) << 8 | i32::from(bytes[2]) << 16) << 8)
                    >> 8;
            (f64::from(value) / 8_388_608.) as f32
        }
        32 => {
            (f64::from(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                / 2_147_483_648.) as f32
        }
        _ => unreachable!("validated WAV sample width"),
    }
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod tests;

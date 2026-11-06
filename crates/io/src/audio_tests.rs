use super::*;

fn chunk(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut result = tag.to_vec();
    result.extend((data.len() as u32).to_le_bytes());
    result.extend(data);
    if !data.len().is_multiple_of(2) {
        result.push(0);
    }
    result
}
fn riff(chunks: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((4 + chunks.iter().map(Vec::len).sum::<usize>() as u32).to_le_bytes());
    bytes.extend(b"WAVE");
    for chunk in chunks {
        bytes.extend(chunk);
    }
    bytes
}
fn format(codec: u16, bits: u16, channels: u16, rate: u32) -> Vec<u8> {
    let alignment = bits / 8 * channels;
    [
        codec.to_le_bytes().to_vec(),
        channels.to_le_bytes().to_vec(),
        rate.to_le_bytes().to_vec(),
        (rate * u32::from(alignment)).to_le_bytes().to_vec(),
        alignment.to_le_bytes().to_vec(),
        bits.to_le_bytes().to_vec(),
    ]
    .concat()
}
fn wav(codec: u16, bits: u16, channels: u16, data: &[u8]) -> Vec<u8> {
    riff(&[
        chunk(b"fmt ", &format(codec, bits, channels, 8_000)),
        chunk(b"data", data),
    ])
}

#[test]
fn pcm_widths_float_and_duration_decode_without_changing_source_bytes() {
    for (codec, bits, data) in [
        (1, 8, vec![0, 128, 255]),
        (
            1,
            16,
            [i16::MIN.to_le_bytes(), [0, 0], i16::MAX.to_le_bytes()].concat(),
        ),
        (1, 24, vec![0, 0, 128, 0, 0, 0, 255, 255, 127]),
        (
            1,
            32,
            [i32::MIN.to_le_bytes(), [0; 4], i32::MAX.to_le_bytes()].concat(),
        ),
        (
            3,
            32,
            [(-1f32).to_le_bytes(), [0; 4], 1f32.to_le_bytes()].concat(),
        ),
    ] {
        let bytes = wav(codec, bits, 1, &data);
        let clip = decode_wav(bytes.clone()).unwrap();
        assert_eq!(clip.bytes, bytes);
        assert_eq!(clip.frames, 3);
        assert_eq!(clip.sample_rate, 8_000);
        assert_eq!(clip.duration(), 3. / 8_000.);
        assert_eq!(clip.peaks[0], WavePeak { min: -1., max: -1. });
        assert_eq!(clip.peaks[1], WavePeak { min: 0., max: 0. });
        assert!(clip.peaks[2].max > 0.99 && clip.peaks[2].max <= 1.);
    }
}

#[test]
fn stereo_envelope_keeps_antiphase_peaks_and_the_last_frame_under_the_bin_budget() {
    let frames = MAX_WAVE_BINS * 3 + 1;
    let mut data = vec![0; frames * 4];
    for offset in [0, (frames - 1) * 4] {
        data[offset..offset + 2].copy_from_slice(&i16::MIN.to_le_bytes());
        data[offset + 2..offset + 4].copy_from_slice(&i16::MAX.to_le_bytes());
    }
    let clip = decode_wav(wav(1, 16, 2, &data)).unwrap();
    assert_eq!(clip.peaks.len(), MAX_WAVE_BINS);
    for peak in [clip.peaks[0], *clip.peaks.last().unwrap()] {
        assert_eq!(peak.min, -1.);
        assert!(peak.max > 0.99);
    }
    assert!(
        clip.peaks
            .iter()
            .all(|peak| peak.min.is_finite() && peak.max.is_finite())
    );
}

#[test]
fn unknown_metadata_odd_padding_and_data_before_format_are_preserved() {
    let mut header = format(3, 32, 1, 48_000);
    header.extend(0u16.to_le_bytes());
    let bytes = riff(&[
        chunk(b"JUNK", &[1, 2, 3]),
        chunk(b"data", &0.25f32.to_le_bytes()),
        chunk(b"fmt ", &header),
        chunk(b"LIST", b"original metadata"),
    ]);
    let clip = decode_wav(bytes.clone()).unwrap();
    assert_eq!(clip.bytes, bytes);
    assert_eq!(
        clip.peaks,
        [WavePeak {
            min: 0.25,
            max: 0.25
        }]
    );
}

#[test]
fn malformed_containers_chunk_lengths_padding_and_duplicate_audio_chunks_are_rejected() {
    let valid = wav(1, 8, 1, &[128]);
    for end in 0..valid.len() {
        assert!(decode_wav(valid[..end].to_vec()).is_err(), "{end}");
    }
    let mut missing_pad = valid.clone();
    missing_pad.pop();
    let length = missing_pad.len() as u32 - 8;
    missing_pad[4..8].copy_from_slice(&length.to_le_bytes());
    assert!(decode_wav(missing_pad).is_err());
    let fmt = chunk(b"fmt ", &format(1, 8, 1, 8_000));
    let data = chunk(b"data", &[128]);
    for bytes in [
        riff(&[fmt.clone(), fmt.clone(), data.clone()]),
        riff(&[fmt.clone(), data.clone(), data.clone()]),
        riff(&[fmt.clone()]),
        riff(&[data]),
        riff(&[chunk(b"JUNK", &[])]),
    ] {
        assert!(decode_wav(bytes).is_err());
    }
    let mut short_extension = format(1, 16, 1, 8_000);
    short_extension.push(0);
    assert!(
        decode_wav(riff(&[
            chunk(b"fmt ", &short_extension),
            chunk(b"data", &[0; 2])
        ]))
        .is_err()
    );
    let mut oversized_extension = format(1, 16, 1, 8_000);
    oversized_extension.extend(8u16.to_le_bytes());
    assert!(
        decode_wav(riff(&[
            chunk(b"fmt ", &oversized_extension),
            chunk(b"data", &[0; 2])
        ]))
        .is_err()
    );
    let mut huge_chunk = valid;
    huge_chunk[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_wav(huge_chunk).is_err());
}

#[test]
fn unsupported_codecs_rates_channels_alignment_and_nonfinite_samples_are_rejected() {
    for (codec, bits, channels, rate) in [
        (2, 16, 1, 8_000),
        (0xfffe, 16, 2, 48_000),
        (1, 12, 1, 8_000),
        (3, 64, 1, 48_000),
        (1, 16, 0, 8_000),
        (1, 16, 3, 48_000),
        (1, 16, 1, 0),
        (1, 16, 1, 7999),
        (1, 16, 1, 192_001),
    ] {
        assert!(
            decode_wav(riff(&[
                chunk(b"fmt ", &format(codec, bits, channels, rate)),
                chunk(b"data", &[0; 8])
            ]))
            .is_err()
        );
    }
    for field in [8, 12] {
        let mut fmt = format(1, 16, 1, 8_000);
        fmt[field] ^= 1;
        assert!(decode_wav(riff(&[chunk(b"fmt ", &fmt), chunk(b"data", &[0; 2])])).is_err());
    }
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(decode_wav(wav(3, 32, 1, &bad.to_le_bytes())).is_err());
    }
    assert!(decode_wav(wav(1, 16, 1, &[0])).is_err());
    assert!(decode_wav(wav(1, 16, 1, &[])).is_err());
    let mut bad_magic = wav(1, 16, 1, &[0; 2]);
    bad_magic[..4].copy_from_slice(b"RIFX");
    assert!(decode_wav(bad_magic).is_err());
}

#[test]
fn file_and_decode_limits_apply_before_large_or_nonregular_inputs_are_loaded() {
    assert!(decode_wav(vec![0; MAX_WAV_BYTES + 1]).is_err());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("peachy-wav-{}-{stamp}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    assert!(load_wav(&directory).is_err());
    let path = directory.join("large.wav");
    File::create(&path)
        .unwrap()
        .set_len(MAX_WAV_BYTES as u64 + 1)
        .unwrap();
    assert!(load_wav(&path).is_err());
    let bytes = wav(1, 16, 1, &[0; 4]);
    std::fs::write(&path, &bytes).unwrap();
    assert_eq!(load_wav(&path).unwrap().bytes, bytes);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_dir_all(directory).unwrap();
}

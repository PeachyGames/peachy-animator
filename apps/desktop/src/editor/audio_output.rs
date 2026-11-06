//! The platform audio boundary. Constructed only after explicitly loading a WAV.
use peachy_io::WavClip;

pub(super) trait AudioPlayer {
    fn position(&self) -> f64;
    fn is_playing(&self) -> bool;
    fn play(&mut self) -> Result<(), String>;
    fn pause(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
    fn seek(&mut self, seconds: f64) -> Result<(), String>;
    fn set_volume(&mut self, gain: f32);
    fn set_looping(&mut self, looping: bool);
}

#[cfg(target_os = "macos")]
pub(super) fn open(clip: &WavClip) -> Result<Option<Box<dyn AudioPlayer>>, String> {
    Ok(Some(Box::new(native::NativePlayer::new(clip)?)))
}

#[cfg(not(target_os = "macos"))]
pub(super) fn open(_clip: &WavClip) -> Result<Option<Box<dyn AudioPlayer>>, String> {
    Ok(None)
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use objc2::{AnyThread, rc::Retained};
    use objc2_app_kit::NSSound;
    use objc2_foundation::NSData;

    pub(super) struct NativePlayer {
        sound: Retained<NSSound>,
        duration: f64,
        decoder_duration: f64,
        paused: bool,
    }

    impl NativePlayer {
        pub(super) fn new(clip: &WavClip) -> Result<Self, String> {
            let data = NSData::with_bytes(&clip.bytes);
            let sound = NSSound::initWithData(NSSound::alloc(), &data)
                .ok_or("macOS could not decode this WAV snapshot")?;
            let duration = sound.duration();
            if !duration.is_finite() || duration <= 0. {
                return Err("The audio decoder reported an invalid duration".into());
            }
            Ok(Self {
                sound,
                duration: clip.duration(),
                decoder_duration: duration,
                paused: false,
            })
        }
    }

    impl AudioPlayer for NativePlayer {
        fn position(&self) -> f64 {
            self.sound.currentTime()
        }
        fn is_playing(&self) -> bool {
            self.sound.isPlaying()
        }
        fn play(&mut self) -> Result<(), String> {
            if self.sound.isPlaying() {
                return Ok(());
            }
            let position = self.sound.currentTime();
            if !position.is_finite() || position < 0. {
                return Err("The audio output reported an invalid position".into());
            }
            let success = if self.paused {
                self.sound.resume()
            } else {
                self.sound.play()
            };
            if !success {
                return Err("The system audio output could not start playback".into());
            }
            if !self.paused {
                // play may start from zero; preserve an explicit seek made while stopped.
                self.sound.setCurrentTime(if position >= self.duration {
                    0.
                } else {
                    position
                });
            }
            self.paused = false;
            Ok(())
        }
        fn pause(&mut self) -> Result<(), String> {
            if self.sound.isPlaying() {
                if !self.sound.pause() {
                    return Err("The audio output could not pause".into());
                }
                self.paused = true;
            }
            Ok(())
        }
        fn stop(&mut self) -> Result<(), String> {
            if (self.sound.isPlaying() || self.paused) && !self.sound.stop() {
                return Err("The audio output could not stop".into());
            }
            self.paused = false;
            self.sound.setCurrentTime(0.);
            Ok(())
        }
        fn seek(&mut self, seconds: f64) -> Result<(), String> {
            if !seconds.is_finite() || seconds < 0. || seconds > self.duration {
                return Err("Audio seek is outside the decoded clip".into());
            }
            self.sound
                .setCurrentTime(seconds.min(self.decoder_duration));
            Ok(())
        }
        fn set_volume(&mut self, gain: f32) {
            self.sound.setVolume(gain);
        }
        fn set_looping(&mut self, looping: bool) {
            self.sound.setLoops(looping);
        }
    }
    impl Drop for NativePlayer {
        fn drop(&mut self) {
            self.sound.setVolume(0.);
            let _ = self.sound.stop();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::{Duration, Instant};

        /// Run deliberately from the coordinator's authenticated GUI context:
        /// cargo test -p peachy-animator native_silent_nssound_transport -- --ignored --nocapture --test-threads=1
        #[test]
        #[ignore = "Requires a usable native audio output; silent, explicit native verification only"]
        fn native_silent_nssound_transport() {
            let rate = 16_000u32;
            let samples = rate * 2;
            let mut bytes = b"RIFF".to_vec();
            bytes.extend((36 + samples * 2).to_le_bytes());
            bytes.extend(b"WAVEfmt ");
            bytes.extend(16u32.to_le_bytes());
            bytes.extend(1u16.to_le_bytes());
            bytes.extend(1u16.to_le_bytes());
            bytes.extend(rate.to_le_bytes());
            bytes.extend((rate * 2).to_le_bytes());
            bytes.extend(2u16.to_le_bytes());
            bytes.extend(16u16.to_le_bytes());
            bytes.extend(b"data");
            bytes.extend((samples * 2).to_le_bytes());
            for index in 0..samples {
                let sample = ((f64::from(index) * std::f64::consts::TAU * 440. / f64::from(rate))
                    .sin()
                    * 4000.) as i16;
                bytes.extend(sample.to_le_bytes());
            }
            let clip = peachy_io::decode_wav(bytes).unwrap();
            let mut player = NativePlayer::new(&clip).unwrap();
            player.set_volume(0.);
            assert_eq!(player.sound.volume(), 0.);
            player.set_looping(true);
            assert!(player.sound.loops());
            player.play().unwrap();
            let deadline = Instant::now() + Duration::from_secs(2);
            while player.position() < 0.04 && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(player.is_playing() && player.position() >= 0.04);
            player.pause().unwrap();
            let paused = player.position();
            std::thread::sleep(Duration::from_millis(80));
            assert!(!player.is_playing() && (player.position() - paused).abs() < 0.025);
            player.seek(0.5).unwrap();
            assert!((player.position() - 0.5).abs() < 0.025);
            player.play().unwrap();
            let deadline = Instant::now() + Duration::from_secs(2);
            while player.position() < 0.55 && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(player.is_playing() && player.position() >= 0.55);
            player.seek(1.95).unwrap();
            std::thread::sleep(Duration::from_millis(300));
            assert!(
                player.is_playing() && player.position() < 1.0,
                "Native loop did not wrap"
            );
            player.set_looping(false);
            assert!(!player.sound.loops());
            player.stop().unwrap();
            assert!(!player.is_playing());
            assert!(player.position().abs() < 0.025);
            println!(
                "{{\"silentVolume\":true,\"nativeClockAdvanced\":true,\"pauseSeekResumeLoopStop\":true,\"audibleListeningAccepted\":false}}"
            );
        }
    }
}

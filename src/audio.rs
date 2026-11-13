//! Weckertöne: synthetisierte Klänge (keine Dateien nötig) oder eigene Audiodatei.

use crate::i18n::t;
use crate::settings::AlarmSound;
use rodio::{OutputStream, Sink, Source};
use std::f32::consts::TAU;
use std::time::{Duration, Instant};

const SR: u32 = 44_100;

/// Ein endlos laufender, synthetisierter Weckton.
struct Synth {
    kind: AlarmSound,
    n: u64,
}

/// Ein angeschlagener Ton mit mehreren abklingenden Teiltönen.
fn strike(dt: f32, freq: f32, partials: &[(f32, f32, f32)]) -> f32 {
    if dt < 0.0 {
        return 0.0;
    }
    let attack = (dt / 0.006).min(1.0);
    let mut s = 0.0;
    for &(ratio, amp, decay) in partials {
        s += (TAU * freq * ratio * dt).sin() * amp * (-decay * dt).exp();
    }
    s * attack
}

impl Synth {
    fn sample(&self, t: f32) -> f32 {
        match self.kind {
            // Gong-artige Glocke: absteigendes Motiv, sanft ausklingend
            AlarmSound::Bell | AlarmSound::Custom => {
                const PERIOD: f32 = 4.2;
                const NOTES: [(f32, f32); 4] =
                    [(0.0, 783.99), (0.5, 659.25), (1.0, 523.25), (1.9, 392.0)];
                const P: [(f32, f32, f32); 5] = [
                    (1.0, 1.0, 1.4),
                    (2.0, 0.45, 2.2),
                    (2.76, 0.28, 3.2),
                    (5.40, 0.12, 5.0),
                    (8.93, 0.05, 8.0),
                ];
                let local = t % PERIOD;
                let mut s = 0.0;
                for &(start, f) in &NOTES {
                    let mut dt = local - start;
                    if dt < 0.0 {
                        dt += PERIOD;
                    }
                    if t - dt >= 0.0 {
                        s += strike(dt, f, &P);
                    }
                }
                s * 0.32
            }
            // Marimba-artige, aufsteigende Melodie
            AlarmSound::Morning => {
                const PERIOD: f32 = 3.0;
                const NOTES: [f32; 7] =
                    [523.25, 659.25, 783.99, 880.0, 1046.5, 880.0, 1318.5];
                const P: [(f32, f32, f32); 3] =
                    [(1.0, 1.0, 3.2), (3.93, 0.32, 10.0), (9.2, 0.07, 22.0)];
                let local = t % PERIOD;
                let mut s = 0.0;
                for (i, &f) in NOTES.iter().enumerate() {
                    let start = i as f32 * 0.21;
                    let mut dt = local - start;
                    if dt < 0.0 {
                        dt += PERIOD;
                    }
                    if t - dt >= 0.0 {
                        s += strike(dt, f, &P);
                    }
                }
                s * 0.30
            }
            // Klassisches Piepen, aber mit weichen Flanken
            AlarmSound::Classic => {
                const PERIOD: f32 = 1.25;
                let local = t % PERIOD;
                let mut env = 0.0;
                for k in 0..4 {
                    let start = k as f32 * 0.16;
                    let dt = local - start;
                    if (0.0..0.1).contains(&dt) {
                        let a = (dt / 0.006).min(1.0);
                        let r = ((0.1 - dt) / 0.012).min(1.0);
                        env = a.min(r);
                    }
                }
                let f = 1046.5;
                let w = (TAU * f * t).sin() + 0.22 * (TAU * 3.0 * f * t).sin();
                w * env * 0.28
            }
        }
    }
}

impl Iterator for Synth {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let t = self.n as f32 / SR as f32;
        self.n += 1;
        // weicher Begrenzer gegen Übersteuerung
        let s = self.sample(t);
        Some(s / (1.0 + s.abs()))
    }
}

impl Source for Synth {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        SR
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// Spielt einen Weckton, wird beim Drop gestoppt.
pub struct AlarmPlayer {
    _stream: OutputStream,
    sink: Sink,
    started: Instant,
    volume: f32,
    fade_secs: f32,
}

impl AlarmPlayer {
    pub fn start(
        sound: AlarmSound,
        custom_path: &str,
        volume: f32,
        fade_secs: f32,
    ) -> Result<Self, String> {
        let (stream, handle) =
            OutputStream::try_default().map_err(|e| format!("{}: {e}", t("Kein Audiogerät", "No audio device")))?;
        let sink = Sink::try_new(&handle).map_err(|e| format!("{}: {e}", t("Audiofehler", "Audio error")))?;

        let mut used_custom = false;
        if sound == AlarmSound::Custom && !custom_path.is_empty() {
            if let Ok(file) = std::fs::File::open(custom_path) {
                if let Ok(dec) = rodio::Decoder::new(std::io::BufReader::new(file)) {
                    sink.append(dec.repeat_infinite());
                    used_custom = true;
                }
            }
        }
        if !used_custom {
            sink.append(Synth { kind: sound, n: 0 });
        }

        let p = Self {
            _stream: stream,
            sink,
            started: Instant::now(),
            volume,
            fade_secs,
        };
        p.update();
        p.sink.play();
        Ok(p)
    }

    pub fn set_volume(&mut self, v: f32) {
        self.volume = v;
    }

    /// Lautstärke-Rampe (sanft lauter werden).
    pub fn update(&self) {
        let ramp = if self.fade_secs > 0.5 {
            let x = (self.started.elapsed().as_secs_f32() / self.fade_secs).clamp(0.0, 1.0);
            0.04 + 0.96 * x * x
        } else {
            1.0
        };
        self.sink.set_volume(self.volume * self.volume * ramp);
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
}

impl Drop for AlarmPlayer {
    fn drop(&mut self) {
        self.sink.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synth_sounds_are_audible_and_clean() {
        for kind in [AlarmSound::Bell, AlarmSound::Morning, AlarmSound::Classic] {
            let s = Synth { kind, n: 0 };
            let samples: Vec<f32> = s.take(SR as usize * 5).collect();
            let peak = samples.iter().fold(0f32, |m, x| m.max(x.abs()));
            assert!(samples.iter().all(|x| x.is_finite()), "{kind:?} NaN");
            assert!(peak > 0.05 && peak < 1.0, "{kind:?} peak {peak}");
        }
    }
}

#[cfg(test)]
mod device_test {
    #[test]
    #[ignore]
    fn opens_audio_device_silently() {
        let p = super::AlarmPlayer::start(crate::settings::AlarmSound::Bell, "", 0.0, 0.0);
        assert!(p.is_ok(), "{:?}", p.err());
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
}

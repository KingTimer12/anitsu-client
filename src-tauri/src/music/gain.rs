//! Volume igualado: ganho de álbum (ReplayGain das tags ou EBU R128 medido).
//!
//! O ganho é fixo por faixa, decidido antes de ela tocar, e limitado pelo pico
//! para nunca cortar o sinal. Só números vão para o disco.

use std::{collections::HashMap, path::Path};

use ebur128::{EbuR128, Mode};
use serde::{Deserialize, Serialize};

use super::decode::Tags;

/// Nível alvo, mais ou menos a referência do ReplayGain.
pub const TARGET_LUFS: f64 = -18.0;
/// Menos que isso tocado não vale como medida da faixa.
const MIN_MEASURED_SECS: f64 = 30.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Measured {
    pub lufs: f64,
    pub peak: f32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LoudnessCache {
    /// Chave: `"caminho|tamanho"`.
    pub tracks: HashMap<String, Measured>,
}

pub fn cache_key(path: &str, size: u64) -> String {
    format!("{path}|{size}")
}

impl LoudnessCache {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        if let Ok(json) = serde_json::to_vec(self) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Média de loudness das faixas medidas do álbum.
    pub fn album_lufs(&self, album_keys: &[String]) -> Option<f64> {
        let v: Vec<f64> = album_keys.iter().filter_map(|k| self.tracks.get(k)).map(|m| m.lufs).collect();
        (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
    }

    /// Maior pico entre as faixas medidas do álbum.
    pub fn album_peak(&self, album_keys: &[String]) -> Option<f32> {
        album_keys.iter().filter_map(|k| self.tracks.get(k)).map(|m| m.peak).reduce(f32::max)
    }
}

fn db_to_linear(db: f64) -> f64 {
    10f64.powf(db / 20.0)
}

/// Ganho linear da faixa: tag de álbum → tag de faixa → média medida do álbum → 1.0,
/// limitado para que `pico × ganho ≤ 1`. Só aumenta quando o pico da própria faixa
/// (ou do álbum, pelas tags) é conhecido; a medida só abaixa.
pub fn track_gain(tags: &Tags, measured_album: Option<(f64, f32)>, enabled: bool) -> f32 {
    if !enabled {
        return 1.0;
    }
    let (gain, peak) = if let Some(db) = tags.album_gain {
        (db_to_linear(db as f64), tags.album_peak.or(tags.track_peak))
    } else if let Some(db) = tags.track_gain {
        (db_to_linear(db as f64), tags.track_peak.or(tags.album_peak))
    } else if let Some((lufs, _)) = measured_album {
        // O pico medido é de outras faixas e não protege esta: medido só abaixa.
        (db_to_linear(TARGET_LUFS - lufs), Some(1.0))
    } else {
        return 1.0;
    };
    // Sem pico conhecido, nunca aumentar: aumentar sem saber o pico pode clipar.
    let limit = peak.filter(|p| *p > 0.0).map_or(1.0, |p| 1.0 / p as f64);
    gain.min(limit) as f32
}

/// Mede o que está tocando (antes do ganho).
pub struct Meter {
    inner: Option<EbuR128>,
    channels: u32,
    rate: u32,
    frames: u64,
}

impl Meter {
    pub fn new(channels: u32, rate: u32) -> Self {
        Self { inner: EbuR128::new(channels, rate, Mode::I | Mode::SAMPLE_PEAK).ok(), channels, rate, frames: 0 }
    }

    pub fn push(&mut self, interleaved: &[f32]) {
        if let Some(m) = &mut self.inner
            && m.add_frames_f32(interleaved).is_ok()
        {
            self.frames += (interleaved.len() / self.channels.max(1) as usize) as u64;
        }
    }

    pub fn secs(&self) -> f64 {
        self.frames as f64 / self.rate.max(1) as f64
    }

    /// Medida da faixa, se tocou o bastante para valer.
    pub fn finish(&self) -> Option<Measured> {
        let m = self.inner.as_ref()?;
        if self.secs() < MIN_MEASURED_SECS {
            return None;
        }
        let lufs = m.loudness_global().ok().filter(|l| l.is_finite())?;
        let peak = (0..self.channels).filter_map(|c| m.sample_peak(c).ok()).fold(0f64, f64::max);
        Some(Measured { lufs, peak: peak as f32 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f64) -> bool {
        (a as f64 - b).abs() < 0.001
    }

    #[test]
    fn prefers_album_tag_over_track_tag() {
        let t = Tags { album_gain: Some(-6.0), track_gain: Some(-3.0), ..Default::default() };
        assert!(close(track_gain(&t, None, true), 0.501));
    }

    #[test]
    fn falls_back_to_track_tag_then_measured_then_unity() {
        let t = Tags { track_gain: Some(-3.0), ..Default::default() };
        assert!(close(track_gain(&t, Some((-12.0, 0.5)), true), 0.708));
        let none = Tags::default();
        assert!(close(track_gain(&none, Some((-12.0, 0.5)), true), 10f64.powf(-6.0 / 20.0)));
        assert_eq!(track_gain(&none, None, true), 1.0);
    }

    #[test]
    fn gain_is_limited_by_peak() {
        let t = Tags { album_gain: Some(6.0), album_peak: Some(0.9), ..Default::default() };
        assert!(close(track_gain(&t, None, true), 1.0 / 0.9));
    }

    #[test]
    fn positive_tag_gain_without_peak_does_not_boost() {
        let t = Tags { album_gain: Some(6.0), ..Default::default() };
        assert_eq!(track_gain(&t, None, true), 1.0);
        let t = Tags { track_gain: Some(-4.0), ..Default::default() };
        assert!(close(track_gain(&t, None, true), 10f64.powf(-4.0 / 20.0)));
    }

    #[test]
    fn measured_fallback_only_attenuates() {
        // Pico medido de outras faixas não protege esta: medido nunca aumenta.
        assert_eq!(track_gain(&Tags::default(), Some((-30.0, 0.3)), true), 1.0);
        assert!(close(track_gain(&Tags::default(), Some((-12.0, 0.5)), true), 10f64.powf(-6.0 / 20.0)));
    }

    #[test]
    fn disabled_is_unity() {
        let t = Tags { album_gain: Some(-6.0), ..Default::default() };
        assert_eq!(track_gain(&t, Some((-10.0, 0.5)), false), 1.0);
    }

    #[test]
    fn meter_measures_sine_near_expected_lufs() {
        // Seno de 1 kHz com amplitude 0,5 nos dois canais ≈ -6 LUFS.
        let rate = 48000;
        let mut m = Meter::new(2, rate);
        let block: Vec<f32> = (0..rate)
            .flat_map(|i| {
                let v = ((i as f64 / rate as f64 * 1000.0 * std::f64::consts::TAU).sin() * 0.5) as f32;
                [v, v]
            })
            .collect();
        for _ in 0..35 {
            m.push(&block);
        }
        let r = m.finish().unwrap();
        assert!((r.lufs - -6.0).abs() < 0.5, "{}", r.lufs);
        assert!((r.peak - 0.5).abs() < 0.01);
    }

    #[test]
    fn short_play_is_not_a_measure() {
        let mut m = Meter::new(1, 48000);
        m.push(&vec![0.25; 48000 * 10]);
        assert_eq!(m.finish(), None);
    }

    #[test]
    fn cache_roundtrip_and_album_average() {
        let mut c = LoudnessCache::default();
        let (a, b) = (cache_key("x/1.flac", 10), cache_key("x/2.flac", 20));
        c.tracks.insert(a.clone(), Measured { lufs: -10.0, peak: 0.8 });
        c.tracks.insert(b.clone(), Measured { lufs: -14.0, peak: 0.9 });
        let keys = vec![a, b, cache_key("x/3.flac", 30)];
        assert_eq!(c.album_lufs(&keys), Some(-12.0));
        assert_eq!(c.album_peak(&keys), Some(0.9));
        let path = std::env::temp_dir().join(format!("higanbana-gain-test-{}.json", std::process::id()));
        c.save(&path);
        let back = LoudnessCache::load(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(back.album_lufs(&keys), Some(-12.0));
        assert_eq!(LoudnessCache::default().album_lufs(&keys), None);
    }
}

//! Sons courts générés en mémoire : aucun fichier audio à distribuer.

use macroquad::audio::{load_sound_from_bytes, play_sound, PlaySoundParams, Sound};

pub struct Sfx {
    hit: Option<Sound>,
    head: Option<Sound>,
    miss: Option<Sound>,
    done: Option<Sound>,
    volume: f32,
}

impl Sfx {
    pub async fn load() -> Self {
        Self {
            hit: tone(880.0, 70, 0.35).await,
            head: tone(1320.0, 90, 0.4).await,
            miss: tone(180.0, 90, 0.3).await,
            done: chime().await,
            volume: 0.7,
        }
    }

    pub fn set_volume(&mut self, percent: u8) {
        self.volume = percent.min(100) as f32 / 100.0;
    }

    pub fn hit(&self) {
        self.play(self.hit.as_ref());
    }

    pub fn head(&self) {
        self.play(self.head.as_ref());
    }

    pub fn miss(&self) {
        self.play(self.miss.as_ref());
    }

    pub fn done(&self) {
        self.play(self.done.as_ref());
    }

    fn play(&self, sound: Option<&Sound>) {
        if self.volume <= 0.0 {
            return;
        }
        if let Some(sound) = sound {
            play_sound(sound, PlaySoundParams { looped: false, volume: self.volume });
        }
    }
}

fn wav(samples: &[i16], rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend(b"RIFF");
    bytes.extend(&(36 + data_len).to_le_bytes());
    bytes.extend(b"WAVE");
    bytes.extend(b"fmt ");
    bytes.extend(&16u32.to_le_bytes());
    bytes.extend(&1u16.to_le_bytes());
    bytes.extend(&1u16.to_le_bytes());
    bytes.extend(&rate.to_le_bytes());
    bytes.extend(&(rate * 2).to_le_bytes());
    bytes.extend(&2u16.to_le_bytes());
    bytes.extend(&16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(&data_len.to_le_bytes());
    for sample in samples {
        bytes.extend(&sample.to_le_bytes());
    }
    bytes
}

fn synth(parts: &[(f32, u32)], volume: f32) -> Vec<u8> {
    let rate = 22050u32;
    let mut samples = Vec::new();
    for &(freq, ms) in parts {
        let count = rate * ms / 1000;
        for i in 0..count {
            let t = i as f32 / rate as f32;
            let env = (1.0 - i as f32 / count as f32).powi(2);
            let wave = (t * freq * std::f32::consts::TAU).sin();
            samples.push((wave * env * volume * i16::MAX as f32) as i16);
        }
    }
    wav(&samples, rate)
}

async fn tone(freq: f32, ms: u32, volume: f32) -> Option<Sound> {
    load_sound_from_bytes(&synth(&[(freq, ms)], volume)).await.ok()
}

async fn chime() -> Option<Sound> {
    load_sound_from_bytes(&synth(&[(660.0, 90), (0.0, 40), (990.0, 140)], 0.35)).await.ok()
}

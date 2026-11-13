//! Réglages et historique, dans des fichiers texte à côté de l'exécutable.

use crate::aim::{Crosshair, Settings, DEFAULT_DURATION, DURATIONS, NORMAL};

pub const PROFILE_FILE: &str = "reglages.txt";
pub const HISTORY_FILE: &str = "historique.txt";

pub struct Profile {
    pub dpi: String,
    pub sens: String,
    pub fov: String,
    pub game: usize,
    pub games: Vec<GameTune>,
    pub train: Settings,
    pub crosshair: Crosshair,
    /// Multiplicateur de sensibilité verticale. 1 = identique à l'horizontale.
    pub vsens: String,
    /// Multiplicateur de sensibilité en visée (zoom). 1 = identique à la sensibilité normale.
    pub ads: String,
    pub invert_y: bool,
    pub volume: u8,
    pub week: i64,
    pub week_done: u32,
    pub week_goal: u32,
}

/// Sensibilité, FOV et multiplicateurs propres à un jeu.
#[derive(Clone)]
pub struct GameTune {
    pub sens: String,
    pub fov: String,
    pub vsens: String,
    pub ads: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            dpi: "800".to_owned(),
            sens: "0.35".to_owned(),
            fov: "103".to_owned(),
            game: 0,
            train: Settings::preset(NORMAL, DEFAULT_DURATION),
            crosshair: Crosshair::default(),
            vsens: "1".to_owned(),
            ads: "1".to_owned(),
            invert_y: false,
            volume: 70,
            week: current_week(),
            week_done: 0,
            week_goal: 5,
            games: (0..crate::games::GAMES.len() + 1)
                .map(|_| GameTune {
                    sens: "0.35".to_owned(),
                    fov: "103".to_owned(),
                    vsens: "1".to_owned(),
                    ads: "1".to_owned(),
                })
                .collect(),
        }
    }
}

pub struct Entry {
    pub time: f64,
    pub mode: usize,
    pub score: f32,
    pub duration: f32,
    pub note: String,
}

/// Numéro de semaine, avec le lundi comme premier jour.
pub fn current_week() -> i64 {
    let seconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let days = seconds.div_euclid(86400.0) as i64;
    (days + 3).div_euclid(7)
}

pub const WEEK_GOALS: &[u32] = &[3, 5, 7, 10];

fn value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| line.strip_prefix(key)?.strip_prefix('=').map(str::trim))
}

fn number(text: &str, key: &str, default: usize, max: usize) -> usize {
    value(text, key).and_then(|v| v.parse().ok()).filter(|v| *v < max).unwrap_or(default)
}

pub fn load_profile() -> Profile {
    let Ok(text) = std::fs::read_to_string(PROFILE_FILE) else {
        return Profile::default();
    };
    let fallback = Profile::default();
    let train = Settings {
        size: number(&text, "size", fallback.train.size, 5),
        speed: number(&text, "speed", fallback.train.speed, 4),
        motion: number(&text, "motion", fallback.train.motion, 3),
        lifetime: number(&text, "lifetime", fallback.train.lifetime, 4),
        zone: number(&text, "zone", fallback.train.zone, 3),
        vertical: value(&text, "vertical").is_some_and(|v| v == "1"),
        duration: number(&text, "duration", fallback.train.duration, DURATIONS.len()),
        humanoid: value(&text, "humanoid").is_some_and(|v| v == "1"),
    };
    let mut profile = Profile {
        dpi: value(&text, "dpi").unwrap_or(&fallback.dpi).to_owned(),
        sens: value(&text, "sens").unwrap_or(&fallback.sens).to_owned(),
        fov: value(&text, "fov").unwrap_or(&fallback.fov).to_owned(),
        vsens: value(&text, "vsens").unwrap_or("1").to_owned(),
        ads: value(&text, "ads").unwrap_or("1").to_owned(),
        invert_y: value(&text, "invert_y").is_some_and(|v| v == "1"),
        volume: value(&text, "volume").and_then(|v| v.parse().ok()).filter(|v: &u8| *v <= 100).unwrap_or(70),
        week: value(&text, "week").and_then(|v| v.parse().ok()).unwrap_or(0),
        week_done: value(&text, "week_done").and_then(|v| v.parse().ok()).unwrap_or(0),
        week_goal: value(&text, "week_goal").and_then(|v| v.parse().ok()).filter(|v| WEEK_GOALS.contains(v)).unwrap_or(5),
        game: value(&text, "game").and_then(|v| v.parse().ok()).unwrap_or(0),
        crosshair: Crosshair {
            color: number(&text, "x_color", 0, 6),
            gap: number(&text, "x_gap", 1, 4),
            length: number(&text, "x_length", 1, 3),
            thick: number(&text, "x_thick", 1, 3),
            dot: value(&text, "x_dot").is_none_or(|v| v == "1"),
        },
        train,
        games: Vec::new(),
    };
    let slots = crate::games::GAMES.len() + 1;
    let sens_fallback = profile.sens.clone();
    let fov_fallback = profile.fov.clone();
    let vsens_fallback = profile.vsens.clone();
    let ads_fallback = profile.ads.clone();
    profile.games = (0..slots)
        .map(|i| GameTune {
            sens: value(&text, &format!("g{i}_sens")).unwrap_or(&sens_fallback).to_owned(),
            fov: value(&text, &format!("g{i}_fov")).unwrap_or(&fov_fallback).to_owned(),
            vsens: value(&text, &format!("g{i}_vsens")).unwrap_or(&vsens_fallback).to_owned(),
            ads: value(&text, &format!("g{i}_ads")).unwrap_or(&ads_fallback).to_owned(),
        })
        .collect();
    let selected = profile.game.min(slots - 1);
    profile.sens = profile.games[selected].sens.clone();
    profile.fov = profile.games[selected].fov.clone();
    profile.vsens = profile.games[selected].vsens.clone();
    profile.ads = profile.games[selected].ads.clone();
    let week = current_week();
    if profile.week != week {
        profile.week = week;
        profile.week_done = 0;
    }
    profile
}

pub fn save_profile(profile: &Profile) {
    let Settings { size, speed, motion, lifetime, zone, vertical, duration, humanoid } = profile.train;
    let Crosshair { color, gap, length, thick, dot } = profile.crosshair;
    let text = format!(
        "dpi={}\nsens={}\nfov={}\nvsens={}\nads={}\ninvert_y={}\nvolume={}\nweek={}\nweek_done={}\nweek_goal={}\ngame={}\nsize={size}\nspeed={speed}\nmotion={motion}\nlifetime={lifetime}\nzone={zone}\nvertical={}\nduration={duration}\nhumanoid={}\nx_color={color}\nx_gap={gap}\nx_length={length}\nx_thick={thick}\nx_dot={}\n",
        profile.dpi,
        profile.sens,
        profile.fov,
        profile.vsens,
        profile.ads,
        if profile.invert_y { "1" } else { "0" },
        profile.volume,
        profile.week,
        profile.week_done,
        profile.week_goal,
        profile.game,
        if vertical { "1" } else { "0" },
        if humanoid { "1" } else { "0" },
        if dot { "1" } else { "0" },
    );
    let mut text = text;
    for (i, tune) in profile.games.iter().enumerate() {
        text += &format!(
            "g{i}_sens={}\ng{i}_fov={}\ng{i}_vsens={}\ng{i}_ads={}\n",
            tune.sens, tune.fov, tune.vsens, tune.ads
        );
    }
    let _ = std::fs::write(PROFILE_FILE, text);
}

pub fn load_history() -> Vec<Entry> {
    let Ok(text) = std::fs::read_to_string(HISTORY_FILE) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(5, ' ');
            let time = parts.next()?.parse().ok()?;
            let mode = parts.next()?.parse().ok()?;
            let score = parts.next()?.parse().ok()?;
            let fourth = parts.next().unwrap_or("");
            let (duration, note) = match fourth.parse::<f32>() {
                Ok(duration) => (duration, parts.next().unwrap_or("").replace('_', " ")),
                Err(_) => (60.0, format!("{fourth} {}", parts.next().unwrap_or("")).trim().replace('_', " ")),
            };
            Some(Entry { time, mode, score, duration, note })
        })
        .filter(|entry| entry.mode < 7 && entry.score.is_finite())
        .collect()
}

pub fn append_history(entry: &Entry) {
    let line = format!(
        "{} {} {} {} {}\n",
        entry.time, entry.mode, entry.score, entry.duration, entry.note.replace(' ', "_")
    );
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(HISTORY_FILE);
    if let Ok(file) = file.as_mut() {
        use std::io::Write;
        let _ = file.write_all(line.as_bytes());
    }
}

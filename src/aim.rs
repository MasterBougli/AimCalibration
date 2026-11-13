use macroquad::prelude::*;
use macroquad::rand::gen_range;

pub const EYE: Vec3 = Vec3::new(0.0, 1.7, 0.0);
/// Zoom appliqué tant que le clic droit est maintenu.
pub const ADS_ZOOM: f32 = 1.5;
pub const FLICK_TARGETS: u32 = 12;
pub const TRACK_DURATION: f32 = 8.0;
const TARGET_DIST: f32 = 12.0;
/// Vitesse de base de la cible de tracking, en degrés par seconde (multipliée selon `SPEEDS`).
const TRACK_BASE_SPEED: (f32, f32) = (18.0, 42.0);

#[derive(PartialEq, Clone, Copy)]
pub enum TestKind {
    /// 12 cibles flick puis 8 s de tracking, pour comparer deux sensibilités.
    Calibration,
    /// Flick puis tracking, la sensibilité change pendant le jeu jusqu'à se stabiliser.
    Live,
    Flick,
    Tracking,
    /// Petites corrections : les cibles apparaissent près du viseur.
    Micro,
    /// Deux cibles, il faut les toucher en alternance.
    Switching,
    /// La cible apparaît après un délai aléatoire : on mesure le temps de réaction.
    Reflex,
    /// La cible sort brièvement de derrière un mur.
    Peek,
    /// Flick qui s'arrête quand le niveau récent est retrouvé.
    Warmup,
}

/// Rayon de la cible (à 12 m).
pub const SIZES: &[(&str, f32)] = &[
    ("Très grande", 0.6),
    ("Grande", 0.45),
    ("Normale", 0.35),
    ("Petite", 0.25),
    ("Minuscule", 0.16),
];
/// Multiplicateur de vitesse de la cible de tracking.
pub const SPEEDS: &[(&str, f32)] = &[("Lente", 0.6), ("Normale", 1.0), ("Rapide", 1.5), ("Très rapide", 2.1)];
/// Dérive latérale des cibles de flick, en degrés par seconde.
pub const MOTIONS: &[(&str, f32)] = &[("Immobiles", 0.0), ("Lentes", 8.0), ("Rapides", 20.0)];
/// Temps avant qu'une cible de flick disparaisse, en secondes (0 = illimité).
pub const LIFETIMES: &[(&str, f32)] = &[("Illimitée", 0.0), ("2 s", 2.0), ("1,2 s", 1.2), ("0,8 s", 0.8)];
/// Demi-largeur horizontale de la zone d'apparition, en degrés.
pub const ZONES: &[(&str, f32)] = &[("Étroite", 25.0), ("Normale", 50.0), ("Large", 80.0)];
pub const DURATIONS: &[(&str, f32)] = &[("30 s", 30.0), ("60 s", 60.0), ("90 s", 90.0), ("2 min", 120.0)];
pub const PRESETS: &[&str] = &["Facile", "Normal", "Difficile", "Expert"];
pub const NORMAL: usize = 1;
pub const DEFAULT_DURATION: usize = 1;

#[derive(Clone, Copy, PartialEq)]
pub struct Settings {
    pub size: usize,
    pub speed: usize,
    pub motion: usize,
    pub lifetime: usize,
    pub zone: usize,
    pub vertical: bool,
    pub duration: usize,
    /// Silhouette avec une tête : un tir à la tête compte comme un headshot.
    pub humanoid: bool,
}

impl Settings {
    pub fn preset(preset: usize, duration: usize) -> Self {
        let p = preset.min(PRESETS.len() - 1);
        Self {
            size: [1, 2, 3, 4][p],
            speed: [0, 1, 2, 3][p],
            motion: [0, 0, 1, 2][p],
            lifetime: [0, 0, 2, 3][p],
            zone: [0, 1, 1, 2][p],
            vertical: p >= 2,
            humanoid: p >= 2,
            duration,
        }
    }

    pub fn matching_preset(&self) -> Option<usize> {
        (0..PRESETS.len()).find(|&p| Self::preset(p, self.duration) == *self)
    }

    fn radius(&self) -> f32 {
        SIZES[self.size].1
    }

    fn duration_secs(&self) -> f32 {
        DURATIONS[self.duration].1
    }

    fn yaw_half(&self) -> f32 {
        ZONES[self.zone].1
    }

    fn pitch_range(&self) -> (f32, f32) {
        let yaw = self.yaw_half();
        (-0.12 * yaw, 6.0 + 0.2 * yaw)
    }
}

pub fn direction(yaw_deg: f32, pitch_deg: f32) -> Vec3 {
    let (y, p) = (yaw_deg.to_radians(), pitch_deg.to_radians());
    vec3(y.sin() * p.cos(), p.sin(), -y.cos() * p.cos())
}

fn target_at(yaw: f32, pitch: f32) -> Vec3 {
    EYE + direction(yaw, pitch) * TARGET_DIST
}

#[derive(Clone, Default)]
pub struct Stats {
    pub shots: u32,
    pub hits: u32,
    pub headshots: u32,
    /// Tirs déclenchés avant l'apparition de la cible, en mode réflexes.
    pub false_starts: u32,
    /// Cibles de flick disparues avant d'avoir été touchées.
    pub expired: u32,
    pub flick_ms: Vec<f32>,
    pub track_on: f32,
    pub track_total: f32,
    /// Sommes des mesures de trajectoire, divisées par `traces`.
    pub overshoot: f32,
    pub tremor: f32,
    pub correction_ms: f32,
    pub traces: u32,
    /// Somme des écarts au moment du clic : positif = trop à droite / trop haut.
    pub bias_yaw: f32,
    pub bias_pitch: f32,
    pub bias_n: u32,
    pub short_hits: u32,
    pub short_shots: u32,
    pub long_hits: u32,
    pub long_shots: u32,
}

impl Stats {
    pub fn accuracy(&self) -> f32 {
        if self.shots == 0 {
            0.0
        } else {
            self.hits as f32 / self.shots as f32
        }
    }

    pub fn avg_flick_ms(&self) -> f32 {
        if self.flick_ms.is_empty() {
            0.0
        } else {
            self.flick_ms.iter().sum::<f32>() / self.flick_ms.len() as f32
        }
    }

    /// 100 points pour 100 % de précision à 600 ms par cible.
    pub fn flick_score(&self) -> f32 {
        let avg = self.avg_flick_ms();
        if avg <= 0.0 {
            return 0.0;
        }
        (self.accuracy() * 100.0 * 600.0 / avg).min(150.0)
    }

    pub fn avg_overshoot(&self) -> f32 {
        if self.traces == 0 { 0.0 } else { self.overshoot / self.traces as f32 }
    }

    pub fn avg_tremor(&self) -> f32 {
        if self.traces == 0 { 0.0 } else { self.tremor / self.traces as f32 }
    }

    pub fn avg_correction(&self) -> f32 {
        if self.traces == 0 { 0.0 } else { self.correction_ms / self.traces as f32 }
    }

    /// Écart entre le quart des flicks les plus rapides et le quart des plus lents.
    pub fn spread_ms(&self) -> Option<f32> {
        if self.flick_ms.len() < 4 {
            return None;
        }
        let mut times = self.flick_ms.clone();
        times.sort_by(f32::total_cmp);
        let quarter = (times.len() / 4).max(1);
        let fast = times[..quarter].iter().sum::<f32>() / quarter as f32;
        let slow = times[times.len() - quarter..].iter().sum::<f32>() / quarter as f32;
        Some(slow - fast)
    }

    pub fn tracking_pct(&self) -> f32 {
        if self.track_total <= 0.0 {
            0.0
        } else {
            self.track_on / self.track_total * 100.0
        }
    }

    pub fn score(&self) -> f32 {
        0.5 * (self.flick_score() + self.tracking_pct())
    }

    /// Score des exercices de clic : précision, volume, et bonus de headshots.
    pub fn click_score(&self) -> f32 {
        self.hits as f32 * self.accuracy() * 100.0 + self.headshots as f32 * 25.0
    }

    pub fn training_score(&self, kind: TestKind) -> f32 {
        match kind {
            TestKind::Tracking => self.tracking_pct() * 100.0,
            TestKind::Reflex => {
                let avg = self.avg_flick_ms();
                if avg <= 0.0 { 0.0 } else { self.hits as f32 * (400.0 / avg) * self.accuracy() * 100.0 }
            }
            TestKind::Calibration => self.score(),
            _ => self.click_score(),
        }
    }

    /// Un écart répété d'un même côté est une habitude, pas une mauvaise sensibilité.
    pub fn direction_note(&self) -> Option<String> {
        if self.bias_n < 6 {
            return None;
        }
        let yaw = self.bias_yaw / self.bias_n as f32;
        let pitch = self.bias_pitch / self.bias_n as f32;
        let mut parts = Vec::new();
        if yaw.abs() >= 0.7 {
            parts.push(if yaw > 0.0 { "trop à droite" } else { "trop à gauche" });
        }
        if pitch.abs() >= 0.7 {
            parts.push(if pitch > 0.0 { "trop haut" } else { "trop bas" });
        }
        Some(if parts.is_empty() {
            "Visée centrée : l'erreur ne vient pas d'un côté.".to_owned()
        } else {
            format!(
                "Biais : {}. Ça ne se corrige pas en changeant la sensibilité.",
                parts.join(", ")
            )
        })
    }

    /// Précision des flicks de moins de 8° et de plus de 18°.
    pub fn distance_note(&self) -> Option<String> {
        let bucket = |hits: u32, shots: u32| (shots >= 3).then(|| hits as f32 / shots as f32 * 100.0);
        match (bucket(self.short_hits, self.short_shots), bucket(self.long_hits, self.long_shots)) {
            (Some(short), Some(long)) => Some(format!("Flicks courts : {short:.0} %  ·  flicks longs : {long:.0} %")),
            (Some(short), None) => Some(format!("Flicks courts : {short:.0} %")),
            (None, Some(long)) => Some(format!("Flicks longs : {long:.0} %")),
            (None, None) => None,
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum Phase {
    Flick,
    TrackWait,
    Track,
    /// Attente avant l'apparition de la cible, en mode réflexes.
    Hidden,
    Done,
}

#[derive(Clone, Copy, PartialEq)]
enum Hit {
    Head,
    Body,
    Miss,
}

#[derive(Clone)]
pub struct MissReplay {
    pub path: Vec<(f32, f32)>,
    pub target: (f32, f32),
    /// Écart, en degrés, entre le viseur et la cible au moment du tir.
    pub error: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Feedback {
    None,
    Hit,
    Headshot,
    Miss,
}

pub struct AimTest {
    pub yaw: f32,
    pub pitch: f32,
    pub kind: TestKind,
    pub phase: Phase,
    pub on_target: bool,
    pub stats: Stats,
    pub feedback: Feedback,
    pub zoomed: bool,
    pub worst: Option<MissReplay>,
    settings: Settings,
    deg_per_count: f32,
    base_deg: f32,
    pub live_mult: f32,
    pub live_step: f32,
    pub live_locked: bool,
    live_sign: f32,
    flick_overshoot: f32,
    flick_reach: f32,
    flick_side: f32,
    flick_span: f32,
    flick_sample: bool,
    live_window: f32,
    live_on: f32,
    live_aim0: f32,
    live_tgt0: f32,
    flick_time_left: f32,
    target_yaw: f32,
    target_pitch: f32,
    target_spawn: f64,
    flick_left: u32,
    drift_vel: f32,
    track_vel: f32,
    track_pitch_vel: f32,
    track_switch_in: f32,
    track_left: f32,
    track_clock: f32,
    bots: [(f32, f32); 2],
    active: usize,
    hidden_until: f64,
    v_ratio: f32,
    flick_budget: f32,
    trace: Vec<(f64, f32, f32)>,
    aim_origin: (f32, f32),
    cover_yaw: f32,
    peek_sign: f32,
    peek_out: f32,
    peek_going_out: bool,
    peek_hold: f32,
    peek_shown_for: f32,
    peek_rewarded: bool,
    ads_ratio: f32,
    invert_y: bool,
    miss_path: Vec<(f32, f32)>,
    miss_target: (f32, f32),
    miss_until: f64,
}

impl AimTest {
    /// Le calibrage ignore `settings` et utilise toujours la difficulté Normale.
    /// `v_ratio` : multiplicateur vertical. `ads_ratio` : multiplicateur en visée (clic droit).
    pub fn new(deg_per_count: f32, kind: TestKind, settings: Settings, v_ratio: f32, ads_ratio: f32, invert_y: bool) -> Self {
        let settings = if matches!(kind, TestKind::Calibration | TestKind::Live) {
            Settings::preset(NORMAL, DEFAULT_DURATION)
        } else {
            settings
        };
        let mut test = Self {
            yaw: 0.0,
            pitch: 0.0,
            kind,
            phase: Phase::Flick,
            on_target: false,
            stats: Stats::default(),
            feedback: Feedback::None,
            zoomed: false,
            worst: None,
            settings,
            deg_per_count,
            base_deg: deg_per_count,
            live_mult: 1.0,
            live_step: 0.08,
            live_locked: false,
            live_sign: 0.0,
            flick_overshoot: 0.0,
            flick_reach: 1.0,
            flick_side: 0.0,
            flick_span: 0.0,
            flick_sample: false,
            live_window: 0.0,
            live_on: 0.0,
            live_aim0: 0.0,
            live_tgt0: 0.0,
            flick_time_left: if kind == TestKind::Live { 40.0 } else { settings.duration_secs() },
            target_yaw: 0.0,
            target_pitch: 0.0,
            target_spawn: get_time(),
            flick_left: FLICK_TARGETS,
            drift_vel: 0.0,
            track_vel: 0.0,
            track_pitch_vel: 0.0,
            track_switch_in: 0.0,
            track_left: match kind {
                TestKind::Calibration => TRACK_DURATION,
                TestKind::Live => 24.0,
                _ => settings.duration_secs(),
            },
            track_clock: 0.0,
            bots: [(0.0, 5.0), (20.0, 5.0)],
            active: 0,
            hidden_until: 0.0,
            v_ratio: v_ratio.clamp(0.1, 2.0),
            flick_budget: match kind {
                TestKind::Warmup => 180.0,
                TestKind::Live => 40.0,
                _ => settings.duration_secs(),
            },
            trace: Vec::new(),
            aim_origin: (0.0, 0.0),
            cover_yaw: -18.0,
            peek_sign: 1.0,
            peek_out: 0.0,
            peek_going_out: true,
            peek_hold: 0.4,
            peek_shown_for: 0.0,
            peek_rewarded: false,
            ads_ratio: ads_ratio.clamp(0.1, 2.0),
            invert_y,
            miss_path: Vec::new(),
            miss_target: (0.0, 0.0),
            miss_until: 0.0,
        };
        if kind == TestKind::Warmup {
            test.flick_time_left = 180.0;
        }
        match kind {
            TestKind::Tracking => test.start_tracking(),
            TestKind::Reflex => test.begin_reflex_wait(),
            TestKind::Switching => test.spawn_switch(),
            TestKind::Micro => test.spawn_micro(),
            TestKind::Peek => test.phase = Phase::Flick,
            _ => test.spawn_flick(),
        }
        test
    }

    pub fn elapsed(&self) -> f32 {
        (self.flick_budget - self.flick_time_left).max(0.0)
    }

    pub fn request_finish(&mut self) {
        self.phase = Phase::Done;
    }

    fn begin_reflex_wait(&mut self) {
        self.phase = Phase::Hidden;
        self.hidden_until = get_time() + gen_range(0.55f32, 2.0) as f64;
    }

    fn timed_drill(&self) -> bool {
        matches!(
            self.kind,
            TestKind::Flick | TestKind::Micro | TestKind::Switching | TestKind::Reflex | TestKind::Peek | TestKind::Warmup
        )
    }

    fn begin_trace(&mut self) {
        self.trace.clear();
        self.aim_origin = (self.yaw, self.pitch);
        let (ty, tp) = self.primary();
        self.flick_span = ((ty - self.yaw).powi(2) + (tp - self.pitch).powi(2)).sqrt();
    }

    fn start_tracking(&mut self) {
        self.phase = Phase::TrackWait;
        self.target_yaw = 0.0;
        self.target_pitch = 5.0;
        self.live_window = 0.0;
        self.live_on = 0.0;
        self.live_aim0 = self.yaw;
        self.live_tgt0 = 0.0;
    }

    fn spawn_flick(&mut self) {
        let yaw_half = self.settings.yaw_half();
        let (pitch_min, pitch_max) = self.settings.pitch_range();
        let min_dist = (yaw_half * 0.4).min(20.0);
        loop {
            let yaw = gen_range(-yaw_half, yaw_half);
            let pitch = gen_range(pitch_min, pitch_max);
            let dist = ((yaw - self.target_yaw).powi(2) + (pitch - self.target_pitch).powi(2)).sqrt();
            if dist >= min_dist {
                self.target_yaw = yaw;
                self.target_pitch = pitch;
                break;
            }
        }
        let drift = MOTIONS[self.settings.motion].1;
        self.drift_vel = if gen_range(0.0f32, 1.0) < 0.5 { drift } else { -drift };
        self.target_spawn = get_time();
        self.begin_trace();
    }

    fn spawn_micro(&mut self) {
        let (min_dist, max_dist) = match self.settings.zone {
            0 => (4.0, 9.0),
            1 => (6.0, 14.0),
            _ => (8.0, 22.0),
        };
        let dist = gen_range(min_dist, max_dist);
        let angle = gen_range(0.0f32, std::f32::consts::TAU);
        self.target_yaw = (self.yaw + dist * angle.cos()).clamp(-80.0, 80.0);
        self.target_pitch = (self.pitch + dist * angle.sin()).clamp(-25.0, 35.0);
        let drift = MOTIONS[self.settings.motion].1;
        self.drift_vel = drift * Self::random_sign();
        self.target_spawn = get_time();
        self.begin_trace();
    }

    fn spawn_switch(&mut self) {
        let half = self.settings.yaw_half().max(12.0);
        self.bots = [
            (gen_range(-half, -half * 0.3), gen_range(-2.0, 12.0)),
            (gen_range(half * 0.3, half), gen_range(-2.0, 12.0)),
        ];
        self.active = 0;
        let drift = MOTIONS[self.settings.motion].1;
        self.drift_vel = if drift == 0.0 { 0.0 } else { drift };
        self.phase = Phase::Flick;
        self.begin_trace();
    }

    fn primary(&self) -> (f32, f32) {
        if self.kind == TestKind::Switching { self.bots[self.active] } else { (self.target_yaw, self.target_pitch) }
    }

    fn random_sign() -> f32 {
        if gen_range(0.0f32, 1.0) < 0.5 { 1.0 } else { -1.0 }
    }

    pub fn target_pos(&self) -> Vec3 {
        let (yaw, pitch) = self.primary();
        target_at(yaw, pitch)
    }

    pub fn target_radius(&self) -> f32 {
        let radius = self.settings.radius();
        if self.phase == Phase::Flick { radius } else { radius * 1.3 }
    }

    fn aiming_sphere(&self, center: Vec3, radius: f32) -> bool {
        let dir = direction(self.yaw, self.pitch);
        let to_target = center - EYE;
        let along = to_target.dot(dir);
        along > 0.0 && to_target.length_squared() - along * along <= radius * radius
    }

    fn classify(&self, pos: Vec3) -> Hit {
        let radius = self.target_radius();
        if self.settings.humanoid && self.kind != TestKind::Tracking {
            let head = pos + Vec3::Y * radius * 2.1;
            if self.aiming_sphere(head, radius * 0.55) {
                return Hit::Head;
            }
            let chest = pos + Vec3::Y * radius * 0.45;
            let hips = pos - Vec3::Y * radius * 0.65;
            if self.aiming_sphere(chest, radius * 0.9) || self.aiming_sphere(hips, radius * 0.8) {
                return Hit::Body;
            }
            return Hit::Miss;
        }
        if self.aiming_sphere(pos, radius) { Hit::Body } else { Hit::Miss }
    }

    fn aiming_at_target(&self) -> bool {
        self.classify(self.target_pos()) != Hit::Miss
    }

    /// Enregistre un tir et renvoie `true` s'il a touché.
    fn register_shot(&mut self) -> bool {
        self.stats.shots += 1;
        match self.classify(self.target_pos()) {
            Hit::Head => {
                self.stats.hits += 1;
                self.stats.headshots += 1;
                self.feedback = Feedback::Headshot;
                true
            }
            Hit::Body => {
                self.stats.hits += 1;
                self.feedback = Feedback::Hit;
                true
            }
            Hit::Miss => {
                self.feedback = Feedback::Miss;
                false
            }
        }
    }

    fn note_aim(&mut self, hit: bool) {
        let (ty, tp) = self.primary();
        self.stats.bias_yaw += self.yaw - ty;
        self.stats.bias_pitch += self.pitch - tp;
        self.stats.bias_n += 1;
        if self.flick_span < 8.0 {
            self.stats.short_shots += 1;
            if hit {
                self.stats.short_hits += 1;
            }
        } else if self.flick_span > 18.0 {
            self.stats.long_shots += 1;
            if hit {
                self.stats.long_hits += 1;
            }
        }
    }

    /// `counts` : déplacement brut de la souris (comptes du capteur) depuis la frame précédente.
    pub fn update(&mut self, counts: Vec2, shot: bool, zoomed: bool, dt: f32) {
        self.feedback = Feedback::None;
        self.zoomed = zoomed;
        let ads = if zoomed { self.ads_ratio } else { 1.0 };
        let y_sign = if self.invert_y { 1.0 } else { -1.0 };
        self.yaw += counts.x * self.deg_per_count * ads;
        self.pitch = (self.pitch + y_sign * counts.y * self.deg_per_count * self.v_ratio * ads).clamp(-89.0, 89.0);

        match self.phase {
            Phase::Hidden => {
                if shot {
                    self.stats.shots += 1;
                    self.stats.false_starts += 1;
                    self.feedback = Feedback::Miss;
                    self.hidden_until = get_time() + 0.45;
                } else if get_time() >= self.hidden_until {
                    self.spawn_flick();
                    self.phase = Phase::Flick;
                }
            }
            Phase::Flick => {
                let peek_hidden = self.kind == TestKind::Peek && self.peek_out < 0.55;
                if shot && peek_hidden {
                    self.stats.shots += 1;
                    self.feedback = Feedback::Miss;
                } else if shot {
                    let hit = self.register_shot();
                    self.note_aim(hit);
                    if !hit {
                        self.remember_miss();
                    }
                    self.analyze_trace();
                    if self.kind == TestKind::Live && self.flick_sample {
                        self.adapt_flick();
                    }
                    if hit {
                        self.stats.flick_ms.push(((get_time() - self.target_spawn) * 1000.0) as f32);
                        self.flick_left = self.flick_left.saturating_sub(1);
                        match self.kind {
                            TestKind::Calibration if self.flick_left == 0 => self.start_tracking(),
                            TestKind::Reflex => self.begin_reflex_wait(),
                            TestKind::Switching => {
                                self.active = 1 - self.active;
                                self.begin_trace();
                            }
                            TestKind::Micro => self.spawn_micro(),
                            TestKind::Peek => {
                                self.peek_rewarded = true;
                                self.peek_going_out = false;
                            }
                            _ => self.spawn_flick(),
                        }
                    }
                }
                self.tick_targets(dt);
                if self.kind == TestKind::Peek && self.peek_out >= 0.55 && self.trace.is_empty() {
                    self.begin_trace();
                }
            }
            Phase::TrackWait => {
                if self.aiming_at_target() {
                    self.phase = Phase::Track;
                    self.track_switch_in = 0.0;
                    self.live_window = 0.0;
                    self.live_on = 0.0;
                    self.live_aim0 = self.yaw;
                    self.live_tgt0 = self.target_yaw;
                } else if self.kind == TestKind::Live {
                    self.live_window += dt;
                    if self.live_window > 4.0 {
                        self.phase = Phase::Track;
                        self.live_window = 0.0;
                        self.live_on = 0.0;
                        self.live_aim0 = self.yaw;
                        self.live_tgt0 = self.target_yaw;
                    }
                }
            }
            Phase::Track => {
                self.track_clock += dt;
                self.track_switch_in -= dt;
                if self.track_switch_in <= 0.0 {
                    let mult = SPEEDS[self.settings.speed].1;
                    let speed = gen_range(TRACK_BASE_SPEED.0, TRACK_BASE_SPEED.1) * mult;
                    self.track_vel = speed * Self::random_sign();
                    self.track_pitch_vel = speed * 0.45 * gen_range(0.3f32, 1.0) * Self::random_sign();
                    self.track_switch_in = gen_range(0.35f32, 1.1) / mult.sqrt();
                }
                let yaw_limit = self.settings.yaw_half() * 0.8;
                if self.target_yaw > yaw_limit {
                    self.track_vel = -self.track_vel.abs();
                } else if self.target_yaw < -yaw_limit {
                    self.track_vel = self.track_vel.abs();
                }
                self.target_yaw += self.track_vel * dt;
                if self.settings.vertical {
                    let (pitch_min, pitch_max) = self.settings.pitch_range();
                    if self.target_pitch > pitch_max {
                        self.track_pitch_vel = -self.track_pitch_vel.abs();
                    } else if self.target_pitch < pitch_min.max(-2.0) {
                        self.track_pitch_vel = self.track_pitch_vel.abs();
                    }
                    self.target_pitch += self.track_pitch_vel * dt;
                } else {
                    self.target_pitch = 5.0 + 3.0 * (self.track_clock * 1.7).sin();
                }

                self.stats.track_total += dt;
                if self.aiming_at_target() {
                    self.stats.track_on += dt;
                }
                self.track_left -= dt;
                if self.kind == TestKind::Live {
                    self.live_window += dt;
                    if self.aiming_at_target() {
                        self.live_on += dt;
                    }
                    if self.live_window >= 1.4 {
                        self.adapt_track();
                        self.live_window = 0.0;
                        self.live_on = 0.0;
                        self.live_aim0 = self.yaw;
                        self.live_tgt0 = self.target_yaw;
                    }
                }
                if self.track_left <= 0.0 {
                    self.phase = Phase::Done;
                }
            }
            Phase::Done => {}
        }

        if self.kind == TestKind::Live && self.phase == Phase::Flick {
            self.flick_time_left -= dt;
            if self.flick_time_left <= 0.0 {
                self.track_left = 24.0;
                self.start_tracking();
            }
        }

        if self.timed_drill() && self.phase != Phase::Done {
            self.flick_time_left -= dt;
            if self.flick_time_left <= 0.0 {
                self.phase = Phase::Done;
            }
        }

        if self.phase == Phase::Flick && self.kind != TestKind::Tracking {
            self.trace.push((get_time(), self.yaw, self.pitch));
            if self.trace.len() > 360 {
                self.trace.drain(0..120);
            }
        }

        self.on_target = matches!(self.phase, Phase::Track | Phase::TrackWait) && self.aiming_at_target();
    }

    fn remember_miss(&mut self) {
        if self.trace.len() < 2 {
            return;
        }
        self.miss_path = self.trace.iter().map(|sample| (sample.1, sample.2)).collect();
        self.miss_path.push((self.yaw, self.pitch));
        self.miss_target = self.primary();
        self.miss_until = get_time() + 1.4;
        let error = {
            let dy = self.yaw - self.miss_target.0;
            let dp = self.pitch - self.miss_target.1;
            (dy * dy + dp * dp).sqrt()
        };
        if self.worst.as_ref().is_none_or(|replay| error > replay.error) {
            self.worst = Some(MissReplay { path: self.miss_path.clone(), target: self.miss_target, error });
        }
    }

    /// Dessine un raté avec la cible au centre. `radius` est la taille du cadre, en pixels.
    pub fn draw_replay(replay: &MissReplay, cx: f32, cy: f32, radius: f32) {
        let mut reach = replay.error.max(2.0);
        for &(yaw, pitch) in &replay.path {
            let dy = yaw - replay.target.0;
            let dp = pitch - replay.target.1;
            reach = reach.max((dy * dy + dp * dp).sqrt());
        }
        let scale = radius / reach;
        let map = |yaw: f32, pitch: f32| vec2(cx + (yaw - replay.target.0) * scale, cy - (pitch - replay.target.1) * scale);
        draw_rectangle(cx - radius - 18.0, cy - radius - 18.0, radius * 2.0 + 36.0, radius * 2.0 + 36.0, Color::new(0.04, 0.05, 0.07, 0.9));
        draw_rectangle_lines(cx - radius - 18.0, cy - radius - 18.0, radius * 2.0 + 36.0, radius * 2.0 + 36.0, 1.5, Color::new(1.0, 1.0, 1.0, 0.15));
        draw_circle(cx, cy, 7.0, Color::from_rgba(255, 210, 70, 255));
        let color = Color::from_rgba(255, 110, 90, 255);
        for window in replay.path.windows(2) {
            let a = map(window[0].0, window[0].1);
            let b = map(window[1].0, window[1].1);
            draw_line(a.x, a.y, b.x, b.y, 2.5, color);
        }
        if let Some(&(yaw, pitch)) = replay.path.last() {
            let end = map(yaw, pitch);
            draw_circle(end.x, end.y, 4.0, color);
        }
    }

    /// Trace le chemin du viseur menant au dernier tir raté, et marque la cible.
    pub fn draw_miss_path(&self, hfov: f32) {
        if get_time() > self.miss_until || self.miss_path.len() < 2 {
            return;
        }
        let alpha = ((self.miss_until - get_time()) / 1.4).clamp(0.0, 1.0) as f32;
        let px = screen_width() / hfov.max(40.0);
        let (cx, cy) = (screen_width() / 2.0, screen_height() / 2.0);
        let project = |yaw: f32, pitch: f32| vec2(cx + (yaw - self.yaw) * px, cy - (pitch - self.pitch) * px);
        let color = Color::new(1.0, 0.42, 0.35, alpha);
        for window in self.miss_path.windows(2) {
            let a = project(window[0].0, window[0].1);
            let b = project(window[1].0, window[1].1);
            draw_line(a.x, a.y, b.x, b.y, 3.0, color);
        }
        let end = project(self.miss_path.last().unwrap().0, self.miss_path.last().unwrap().1);
        draw_circle(end.x, end.y, 4.0, color);
        let target = project(self.miss_target.0, self.miss_target.1);
        draw_circle_lines(target.x, target.y, 14.0, 2.0, Color::new(1.0, 0.85, 0.3, alpha));
    }

    fn apply_bias(&mut self, bias: f32) {
        if self.live_locked {
            return;
        }
        if bias.abs() < 0.2 {
            self.live_step = (self.live_step * 0.9).max(0.008);
        } else {
            let sign = bias.signum();
            if self.live_sign != 0.0 && sign != self.live_sign {
                self.live_step *= 0.55;
            }
            self.live_sign = sign;
            self.live_mult = (self.live_mult * (1.0 + sign * self.live_step)).clamp(0.45, 2.2);
            self.deg_per_count = self.base_deg * self.live_mult;
        }
        if self.live_step < 0.012 {
            self.live_locked = true;
        }
    }

    /// Trop de dépassement : sensibilité trop haute. Mouvement qui s'arrête avant la cible : trop basse.
    fn adapt_flick(&mut self) {
        let sideways = self.flick_side > 2.0
            && self.flick_overshoot < self.flick_side
            && (0.75..1.2).contains(&self.flick_reach);
        if sideways {
            return;
        }
        let bias = if self.flick_overshoot > 2.0 {
            -1.0
        } else if self.flick_reach < 0.85 {
            1.0
        } else if self.flick_overshoot > 0.9 {
            -0.45
        } else if self.flick_reach < 0.96 {
            0.45
        } else {
            0.0
        };
        self.apply_bias(bias);
    }

    /// Le viseur qui traîne derrière la cible accélère la souris. Un viseur qui la double la ralentit.
    fn adapt_track(&mut self) {
        let aim = self.yaw - self.live_aim0;
        let target = self.target_yaw - self.live_tgt0;
        let on_target = self.live_on / self.live_window.max(0.1);
        if target.abs() < 4.0 {
            return;
        }
        if on_target > 0.72 {
            self.apply_bias(0.0);
            return;
        }
        let ratio = aim / target;
        let bias = if ratio > 1.35 || ratio < 0.0 {
            -1.0
        } else if ratio < 0.7 {
            1.0
        } else {
            0.0
        };
        self.apply_bias(bias);
    }

    fn analyze_trace(&mut self) {
        self.flick_sample = false;
        let target = self.primary();
        let origin = self.aim_origin;
        let v = (target.0 - origin.0, target.1 - origin.1);
        let vlen2 = v.0 * v.0 + v.1 * v.1;
        if vlen2 < 0.36 || self.trace.len() < 4 {
            return;
        }
        let mut max_t = 0.0f32;
        for &(_, y, p) in &self.trace {
            max_t = max_t.max(((y - origin.0) * v.0 + (p - origin.1) * v.1) / vlen2);
        }
        let overshoot = ((max_t - 1.0).max(0.0) * vlen2.sqrt()).min(40.0);
        let mut side = 0.0f32;
        for &(_, y, p) in &self.trace {
            let t = ((y - origin.0) * v.0 + (p - origin.1) * v.1) / vlen2;
            let dx = y - (origin.0 + t * v.0);
            let dy = p - (origin.1 + t * v.1);
            side = side.max((dx * dx + dy * dy).sqrt());
        }
        self.flick_overshoot = overshoot;
        self.flick_reach = max_t;
        self.flick_side = side;
        self.flick_sample = true;

        let now = get_time();
        let recent: Vec<_> = self.trace.iter().copied().filter(|sample| now - sample.0 <= 0.22).collect();
        let mut reversals = 0.0f32;
        let mut reversals_n = 0u32;
        for window in recent.windows(3) {
            let a = (window[1].1 - window[0].1, window[1].2 - window[0].2);
            let b = (window[2].1 - window[1].1, window[2].2 - window[1].2);
            if a.0 * b.0 + a.1 * b.1 < 0.0 {
                reversals += (b.0 * b.0 + b.1 * b.1).sqrt();
                reversals_n += 1;
            }
        }
        let tremor = if reversals_n == 0 { 0.0 } else { reversals / reversals_n as f32 };

        let correction = self
            .trace
            .iter()
            .find(|sample| {
                let dy = sample.1 - target.0;
                let dp = sample.2 - target.1;
                dy * dy + dp * dp < 6.25
            })
            .map(|sample| ((now - sample.0) * 1000.0) as f32)
            .unwrap_or(0.0);

        self.stats.overshoot += overshoot;
        self.stats.tremor += tremor;
        self.stats.correction_ms += correction;
        self.stats.traces += 1;
        self.trace.clear();
        self.aim_origin = (self.yaw, self.pitch);
    }

    fn tick_peek(&mut self, dt: f32) {
        let mult = SPEEDS[self.settings.speed].1;
        if self.peek_hold > 0.0 {
            self.peek_hold -= dt;
            self.peek_out = 0.0;
            self.target_yaw = self.cover_yaw;
            self.target_pitch = 3.5;
            return;
        }
        let rate = 1.7 * mult;
        if self.peek_going_out {
            self.peek_out = (self.peek_out + rate * dt).min(1.0);
            if self.peek_out >= 1.0 {
                self.peek_shown_for += dt;
                if self.peek_shown_for >= 0.6 / mult {
                    self.peek_going_out = false;
                }
            }
        } else {
            self.peek_out = (self.peek_out - rate * 1.3 * dt).max(0.0);
            if self.peek_out <= 0.0 {
                if !self.peek_rewarded {
                    self.stats.expired += 1;
                }
                self.peek_rewarded = false;
                self.peek_shown_for = 0.0;
                self.peek_sign = -self.peek_sign;
                self.cover_yaw = self.peek_sign * gen_range(14.0, 30.0);
                self.peek_hold = gen_range(0.3, 0.9);
                self.peek_going_out = true;
                self.trace.clear();
            }
        }
        self.target_yaw = self.cover_yaw - self.peek_sign * self.peek_out * 15.0;
        self.target_pitch = 3.5;
    }

    fn tick_targets(&mut self, dt: f32) {
        if self.kind == TestKind::Peek {
            self.tick_peek(dt);
            return;
        }
        let yaw_half = self.settings.yaw_half();
        if self.kind == TestKind::Switching {
            for (i, bot) in self.bots.iter_mut().enumerate() {
                let dir = if i == 0 { 1.0 } else { -1.0 };
                bot.0 += self.drift_vel * dir * dt;
                if bot.0 > yaw_half {
                    self.drift_vel = -self.drift_vel.abs();
                } else if bot.0 < -yaw_half {
                    self.drift_vel = self.drift_vel.abs();
                }
            }
            return;
        }
        if self.kind == TestKind::Reflex {
            if get_time() - self.target_spawn > 1.5 {
                self.stats.expired += 1;
                self.begin_reflex_wait();
            }
            return;
        }
        if self.target_yaw > yaw_half {
            self.drift_vel = -self.drift_vel.abs();
        } else if self.target_yaw < -yaw_half {
            self.drift_vel = self.drift_vel.abs();
        }
        self.target_yaw += self.drift_vel * dt;

        let lifetime = LIFETIMES[self.settings.lifetime].1;
        if lifetime > 0.0 && get_time() - self.target_spawn > lifetime as f64 {
            self.stats.expired += 1;
            if self.kind == TestKind::Micro { self.spawn_micro() } else { self.spawn_flick() }
        }
    }

    pub fn status(&self) -> String {
        match self.phase {
            Phase::Hidden => format!("Réflexes — {:.0} s  ·  attendez la cible", self.flick_time_left.max(0.0).ceil()),
            Phase::Flick if self.kind == TestKind::Live => format!(
                "Flick — {:.0} s  ·  {}",
                self.flick_time_left.max(0.0).ceil(),
                if self.live_locked { "sensibilité stabilisée" } else { "la sensibilité s'ajuste" }
            ),
            Phase::Track if self.kind == TestKind::Live => format!(
                "Tracking — {:.0} s  ·  {}",
                self.track_left.max(0.0).ceil(),
                if self.live_locked { "sensibilité stabilisée" } else { "la sensibilité s'ajuste" }
            ),
            Phase::Flick if self.timed_drill() => {
                let label = match self.kind {
                    TestKind::Micro => "Micro",
                    TestKind::Switching => "Switching",
                    TestKind::Reflex => "Réflexes",
                    TestKind::Peek => "Peek",
                    TestKind::Warmup => "Échauffement",
                    _ => "Flick",
                };
                format!("{label} — {:.0} s  ·  {} touchées", self.flick_time_left.max(0.0).ceil(), self.stats.hits)
            }
            Phase::Flick => format!("Flick — {} cibles restantes", self.flick_left),
            Phase::TrackWait => "Tracking — placez le viseur sur la cible".to_owned(),
            Phase::Track => format!("Tracking — {:.1} s", self.track_left.max(0.0)),
            Phase::Done => String::new(),
        }
    }
}

pub fn camera(yaw: f32, pitch: f32, hfov_deg: f32) -> Camera3D {
    let aspect = screen_width() / screen_height();
    let vfov = 2.0 * ((hfov_deg.to_radians() / 2.0).tan() / aspect).atan();
    Camera3D {
        position: EYE,
        target: EYE + direction(yaw, pitch),
        up: Vec3::Y,
        fovy: vfov,
        aspect: Some(aspect),
        ..Default::default()
    }
}

pub fn draw_world() {
    clear_background(Color::from_rgba(18, 20, 28, 255));
    draw_plane(vec3(0.0, -0.01, 0.0), vec2(40.0, 40.0), None, Color::from_rgba(34, 38, 50, 255));
    draw_grid(40, 2.0, Color::from_rgba(80, 88, 112, 255), Color::from_rgba(50, 55, 70, 255));

    let wall = Color::from_rgba(28, 32, 44, 255);
    draw_cube(vec3(0.0, 8.0, -40.0), vec3(80.0, 16.0, 0.5), None, wall);
    draw_cube(vec3(0.0, 8.0, 40.0), vec3(80.0, 16.0, 0.5), None, wall);
    draw_cube(vec3(-40.0, 8.0, 0.0), vec3(0.5, 16.0, 80.0), None, wall);
    draw_cube(vec3(40.0, 8.0, 0.0), vec3(0.5, 16.0, 80.0), None, wall);

    for i in 0..12 {
        let pos = direction(i as f32 * 30.0, 0.0) * 24.0 + vec3(0.0, 4.0, 0.0);
        let color = if i == 0 {
            Color::from_rgba(70, 130, 200, 255)
        } else {
            Color::from_rgba(60, 66, 86, 255)
        };
        draw_cube(pos, vec3(1.0, 8.0, 1.0), None, color);
    }
}

fn draw_bot(pos: Vec3, radius: f32, color: Color, humanoid: bool) {
    if humanoid {
        let head = pos + Vec3::Y * radius * 2.1;
        draw_cube(pos, vec3(radius * 1.35, radius * 2.7, radius * 0.7), None, color);
        draw_sphere(head, radius * 0.55, None, Color::from_rgba(255, 214, 180, 255));
    } else {
        draw_sphere(pos, radius, None, color);
    }
}

pub fn draw_target(test: &AimTest) {
    if test.kind == TestKind::Peek {
        let cover = EYE + direction(test.cover_yaw, 0.0) * 7.5 + vec3(0.0, 1.4, 0.0);
        draw_cube(cover, vec3(0.55, 3.6, 2.4), None, Color::from_rgba(48, 54, 72, 255));
        if test.peek_out < 0.2 {
            return;
        }
    }
    if test.phase == Phase::Hidden || test.phase == Phase::Done {
        return;
    }
    let radius = test.target_radius();
    let humanoid = test.settings.humanoid && test.kind != TestKind::Tracking;
    if test.kind == TestKind::Switching {
        for (i, &(yaw, pitch)) in test.bots.iter().enumerate() {
            let active = i == test.active;
            let color = if active {
                Color::from_rgba(235, 64, 72, 255)
            } else {
                Color::from_rgba(120, 70, 80, 255)
            };
            draw_bot(target_at(yaw, pitch), radius, color, humanoid);
        }
        return;
    }
    let color = match test.phase {
        Phase::Flick => Color::from_rgba(235, 64, 72, 255),
        _ if test.on_target => Color::from_rgba(70, 220, 120, 255),
        _ => Color::from_rgba(245, 160, 50, 255),
    };
    draw_bot(test.target_pos(), radius, color, humanoid);
}

pub const XHAIR_COLORS: &[(&str, Color)] = &[
    ("Turquoise", Color::from_rgba(0, 255, 200, 255)),
    ("Blanc", Color::from_rgba(245, 245, 245, 255)),
    ("Jaune", Color::from_rgba(255, 220, 60, 255)),
    ("Vert", Color::from_rgba(80, 255, 120, 255)),
    ("Rouge", Color::from_rgba(255, 70, 70, 255)),
    ("Cyan", Color::from_rgba(80, 200, 255, 255)),
];
pub const XHAIR_GAPS: &[(&str, f32)] = &[("Serré", 2.0), ("Normal", 5.0), ("Large", 9.0), ("Très large", 14.0)];
pub const XHAIR_LENGTHS: &[(&str, f32)] = &[("Court", 5.0), ("Moyen", 9.0), ("Long", 15.0)];
pub const XHAIR_THICK: &[(&str, f32)] = &[("Fin", 1.0), ("Normal", 2.0), ("Épais", 4.0)];

#[derive(Clone, Copy)]
pub struct Crosshair {
    pub color: usize,
    pub gap: usize,
    pub length: usize,
    pub thick: usize,
    pub dot: bool,
}

impl Default for Crosshair {
    fn default() -> Self {
        Self { color: 0, gap: 1, length: 1, thick: 1, dot: true }
    }
}

pub fn draw_crosshair(crosshair: &Crosshair) {
    draw_crosshair_at(crosshair, screen_width() / 2.0, screen_height() / 2.0);
}

pub fn draw_crosshair_at(crosshair: &Crosshair, cx: f32, cy: f32) {
    let color = XHAIR_COLORS[crosshair.color % XHAIR_COLORS.len()].1;
    let gap = XHAIR_GAPS[crosshair.gap % XHAIR_GAPS.len()].1;
    let len = XHAIR_LENGTHS[crosshair.length % XHAIR_LENGTHS.len()].1;
    let w = XHAIR_THICK[crosshair.thick % XHAIR_THICK.len()].1;
    draw_line(cx - gap - len, cy, cx - gap, cy, w, color);
    draw_line(cx + gap, cy, cx + gap + len, cy, w, color);
    draw_line(cx, cy - gap - len, cx, cy - gap, w, color);
    draw_line(cx, cy + gap, cx, cy + gap + len, w, color);
    if crosshair.dot {
        draw_circle(cx, cy, (w * 0.8).max(1.2), color);
    }
}

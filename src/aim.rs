use macroquad::prelude::*;
use macroquad::rand::gen_range;

pub const EYE: Vec3 = Vec3::new(0.0, 1.7, 0.0);
pub const FLICK_TARGETS: u32 = 12;
pub const TRACK_DURATION: f32 = 8.0;
const TARGET_DIST: f32 = 12.0;
/// Vitesse de base de la cible de tracking, en degrés par seconde (multipliée selon `SPEEDS`).
const TRACK_BASE_SPEED: (f32, f32) = (18.0, 42.0);

#[derive(PartialEq, Clone, Copy)]
pub enum TestKind {
    /// 12 cibles flick puis 8 s de tracking, pour comparer deux sensibilités.
    Calibration,
    Flick,
    Tracking,
    /// Petites corrections : les cibles apparaissent près du viseur.
    Micro,
    /// Deux cibles, il faut les toucher en alternance.
    Switching,
    /// La cible apparaît après un délai aléatoire : on mesure le temps de réaction.
    Reflex,
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
    settings: Settings,
    deg_per_count: f32,
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
}

impl AimTest {
    /// Le calibrage ignore `settings` et utilise toujours la difficulté Normale.
    pub fn new(deg_per_count: f32, kind: TestKind, settings: Settings) -> Self {
        let settings = if kind == TestKind::Calibration {
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
            settings,
            deg_per_count,
            flick_time_left: settings.duration_secs(),
            target_yaw: 0.0,
            target_pitch: 0.0,
            target_spawn: get_time(),
            flick_left: FLICK_TARGETS,
            drift_vel: 0.0,
            track_vel: 0.0,
            track_pitch_vel: 0.0,
            track_switch_in: 0.0,
            track_left: if kind == TestKind::Calibration { TRACK_DURATION } else { settings.duration_secs() },
            track_clock: 0.0,
            bots: [(0.0, 5.0), (20.0, 5.0)],
            active: 0,
            hidden_until: 0.0,
        };
        match kind {
            TestKind::Tracking => test.start_tracking(),
            TestKind::Reflex => test.begin_reflex_wait(),
            TestKind::Switching => test.spawn_switch(),
            TestKind::Micro => test.spawn_micro(),
            _ => test.spawn_flick(),
        }
        test
    }

    fn begin_reflex_wait(&mut self) {
        self.phase = Phase::Hidden;
        self.hidden_until = get_time() + gen_range(0.55f32, 2.0) as f64;
    }

    fn timed_drill(&self) -> bool {
        matches!(self.kind, TestKind::Flick | TestKind::Micro | TestKind::Switching | TestKind::Reflex)
    }

    fn start_tracking(&mut self) {
        self.phase = Phase::TrackWait;
        self.target_yaw = 0.0;
        self.target_pitch = 5.0;
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

    /// `counts` : déplacement brut de la souris (comptes du capteur) depuis la frame précédente.
    pub fn update(&mut self, counts: Vec2, shot: bool, dt: f32) {
        self.feedback = Feedback::None;
        self.yaw += counts.x * self.deg_per_count;
        self.pitch = (self.pitch - counts.y * self.deg_per_count).clamp(-89.0, 89.0);

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
                if shot && self.register_shot() {
                    self.stats.flick_ms.push(((get_time() - self.target_spawn) * 1000.0) as f32);
                    self.flick_left = self.flick_left.saturating_sub(1);
                    match self.kind {
                        TestKind::Calibration if self.flick_left == 0 => self.start_tracking(),
                        TestKind::Reflex => self.begin_reflex_wait(),
                        TestKind::Switching => self.active = 1 - self.active,
                        TestKind::Micro => self.spawn_micro(),
                        _ => self.spawn_flick(),
                    }
                }
                self.tick_targets(dt);
            }
            Phase::TrackWait => {
                if self.aiming_at_target() {
                    self.phase = Phase::Track;
                    self.track_switch_in = 0.0;
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
                if self.track_left <= 0.0 {
                    self.phase = Phase::Done;
                }
            }
            Phase::Done => {}
        }

        if self.timed_drill() && self.phase != Phase::Done {
            self.flick_time_left -= dt;
            if self.flick_time_left <= 0.0 {
                self.phase = Phase::Done;
            }
        }

        self.on_target = matches!(self.phase, Phase::Track | Phase::TrackWait) && self.aiming_at_target();
    }

    fn tick_targets(&mut self, dt: f32) {
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
            Phase::Flick if self.timed_drill() => {
                let label = match self.kind {
                    TestKind::Micro => "Micro",
                    TestKind::Switching => "Switching",
                    TestKind::Reflex => "Réflexes",
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

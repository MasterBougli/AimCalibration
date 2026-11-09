use macroquad::prelude::*;
use macroquad::rand::gen_range;

pub const EYE: Vec3 = Vec3::new(0.0, 1.7, 0.0);
pub const FLICK_TARGETS: u32 = 12;
pub const TRACK_DURATION: f32 = 8.0;

const TARGET_DIST: f32 = 12.0;
const FLICK_RADIUS: f32 = 0.35;
const TRACK_RADIUS: f32 = 0.45;
const TRACK_YAW_LIMIT: f32 = 40.0;

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
}

#[derive(PartialEq, Clone, Copy)]
pub enum Phase {
    Flick,
    TrackWait,
    Track,
    Done,
}

pub struct AimTest {
    pub yaw: f32,
    pub pitch: f32,
    pub phase: Phase,
    pub on_target: bool,
    pub stats: Stats,
    deg_per_count: f32,
    target_yaw: f32,
    target_pitch: f32,
    target_spawn: f64,
    flick_left: u32,
    track_vel: f32,
    track_switch_in: f32,
    track_left: f32,
    track_clock: f32,
}

impl AimTest {
    pub fn new(deg_per_count: f32) -> Self {
        let mut test = Self {
            yaw: 0.0,
            pitch: 0.0,
            phase: Phase::Flick,
            on_target: false,
            stats: Stats::default(),
            deg_per_count,
            target_yaw: 0.0,
            target_pitch: 0.0,
            target_spawn: get_time(),
            flick_left: FLICK_TARGETS,
            track_vel: 0.0,
            track_switch_in: 0.0,
            track_left: TRACK_DURATION,
            track_clock: 0.0,
        };
        test.spawn_flick();
        test
    }

    fn spawn_flick(&mut self) {
        loop {
            let yaw = gen_range(-50.0f32, 50.0);
            let pitch = gen_range(-6.0f32, 16.0);
            let dist = ((yaw - self.target_yaw).powi(2) + (pitch - self.target_pitch).powi(2)).sqrt();
            if dist >= 20.0 {
                self.target_yaw = yaw;
                self.target_pitch = pitch;
                break;
            }
        }
        self.target_spawn = get_time();
    }

    pub fn target_pos(&self) -> Vec3 {
        target_at(self.target_yaw, self.target_pitch)
    }

    pub fn target_radius(&self) -> f32 {
        if self.phase == Phase::Flick {
            FLICK_RADIUS
        } else {
            TRACK_RADIUS
        }
    }

    fn aiming_at_target(&self) -> bool {
        let dir = direction(self.yaw, self.pitch);
        let to_target = self.target_pos() - EYE;
        let along = to_target.dot(dir);
        let r = self.target_radius();
        along > 0.0 && to_target.length_squared() - along * along <= r * r
    }

    /// `counts` : déplacement brut de la souris (comptes du capteur) depuis la frame précédente.
    pub fn update(&mut self, counts: Vec2, shot: bool, dt: f32) {
        self.yaw += counts.x * self.deg_per_count;
        self.pitch = (self.pitch - counts.y * self.deg_per_count).clamp(-89.0, 89.0);

        match self.phase {
            Phase::Flick => {
                if shot {
                    self.stats.shots += 1;
                    if self.aiming_at_target() {
                        self.stats.hits += 1;
                        self.stats.flick_ms.push(((get_time() - self.target_spawn) * 1000.0) as f32);
                        self.flick_left -= 1;
                        if self.flick_left == 0 {
                            self.phase = Phase::TrackWait;
                            self.target_yaw = 0.0;
                            self.target_pitch = 5.0;
                        } else {
                            self.spawn_flick();
                        }
                    }
                }
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
                    let speed = gen_range(18.0f32, 42.0);
                    self.track_vel = if gen_range(0.0f32, 1.0) < 0.5 { speed } else { -speed };
                    self.track_switch_in = gen_range(0.35f32, 1.1);
                }
                if self.target_yaw > TRACK_YAW_LIMIT {
                    self.track_vel = -self.track_vel.abs();
                } else if self.target_yaw < -TRACK_YAW_LIMIT {
                    self.track_vel = self.track_vel.abs();
                }
                self.target_yaw += self.track_vel * dt;
                self.target_pitch = 5.0 + 3.0 * (self.track_clock * 1.7).sin();

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

        self.on_target = self.phase != Phase::Flick && self.aiming_at_target();
    }

    pub fn status(&self) -> String {
        match self.phase {
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

pub fn draw_target(test: &AimTest) {
    let color = match test.phase {
        Phase::Flick => Color::from_rgba(235, 64, 72, 255),
        _ if test.on_target => Color::from_rgba(70, 220, 120, 255),
        _ => Color::from_rgba(245, 160, 50, 255),
    };
    draw_sphere(test.target_pos(), test.target_radius(), None, color);
}

pub fn draw_crosshair() {
    let (cx, cy) = (screen_width() / 2.0, screen_height() / 2.0);
    let color = Color::from_rgba(0, 255, 200, 255);
    let (gap, len, w) = (4.0, 8.0, 2.0);
    draw_line(cx - gap - len, cy, cx - gap, cy, w, color);
    draw_line(cx + gap, cy, cx + gap + len, cy, w, color);
    draw_line(cx, cy - gap - len, cx, cy - gap, w, color);
    draw_line(cx, cy + gap, cx, cy + gap + len, w, color);
    draw_circle(cx, cy, 1.5, color);
}

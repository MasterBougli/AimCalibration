#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod aim;
mod games;
mod mouse_info;

use aim::{AimTest, FLICK_TARGETS, Phase, Stats, TRACK_DURATION};
use games::GAMES;
use mouse_info::MouseInfo;
use macroquad::miniquad;
use macroquad::prelude::*;
use macroquad::rand::{gen_range, srand};

const ROUNDS: usize = 6;
const RESULTS_FILE: &str = "resultats_sensibilite.txt";

fn window_conf() -> Conf {
    Conf {
        window_title: "Aim Calibration".to_owned(),
        fullscreen: true,
        high_dpi: true,
        sample_count: 4,
        platform: miniquad::conf::Platform {
            swap_interval: Some(0),
            ..Default::default()
        },
        ..Default::default()
    }
}

struct Ui {
    font: Option<Font>,
}

impl Ui {
    fn load() -> Self {
        let font = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf")
            .ok()
            .and_then(|bytes| load_ttf_font_from_bytes(&bytes).ok());
        Self { font }
    }

    fn scale() -> f32 {
        screen_height() / 1080.0
    }

    fn width(&self, s: &str, size: f32) -> f32 {
        measure_text(s, self.font.as_ref(), (size * Self::scale()) as u16, 1.0).width
    }

    fn text(&self, s: &str, x: f32, y: f32, size: f32, color: Color) {
        let params = TextParams {
            font: self.font.as_ref(),
            font_size: (size * Self::scale()) as u16,
            color,
            ..Default::default()
        };
        draw_text_ex(s, x, y * Self::scale(), params);
    }

    fn center(&self, s: &str, y: f32, size: f32, color: Color) {
        self.center_at(s, screen_width() / 2.0, y, size, color);
    }

    fn center_at(&self, s: &str, cx: f32, y: f32, size: f32, color: Color) {
        self.text(s, cx - self.width(s, size) / 2.0, y, size, color);
    }
}

const WHITE_SOFT: Color = Color::new(0.92, 0.93, 0.96, 1.0);
const GREY: Color = Color::new(0.6, 0.63, 0.7, 1.0);
const ACCENT: Color = Color::new(0.0, 1.0, 0.78, 1.0);
const GOLD: Color = Color::new(1.0, 0.8, 0.3, 1.0);
const RED: Color = Color::new(1.0, 0.4, 0.4, 1.0);

#[derive(PartialEq, Clone, Copy)]
enum Screen {
    Setup,
    Intro,
    Playing,
    Paused,
    RoundResult,
    Final,
    Help,
    DpiMeasure,
}

const MEASURE_REFS: &[(&str, f32)] = &[
    ("Carte bancaire (8,56 cm)", 8.56),
    ("Règle — 10 cm", 10.0),
    ("Règle — 20 cm", 20.0),
];

struct App {
    ui: Ui,
    screen: Screen,
    field: usize,
    dpi_text: String,
    sens_text: String,
    fov_text: String,
    game: usize,
    error: Option<String>,
    notice: Option<String>,
    dpi: f32,
    fov: f32,
    base_cm: f32,
    round: usize,
    // Intervalle de multiplicateurs de sensibilité testés (1.0 = sensibilité de départ).
    lo: f32,
    hi: f32,
    order: [f32; 2],
    slot: usize,
    results: [Option<Stats>; 2],
    test: Option<AimTest>,
    last_mouse: Option<Vec2>,
    measure_ref: usize,
    measures: Vec<f32>,
    measuring: bool,
    measure_counts: Vec2,
    measure_error: Option<String>,
    mice: Vec<MouseInfo>,
}

impl App {
    fn new() -> Self {
        Self {
            ui: Ui::load(),
            screen: Screen::Setup,
            field: 0,
            dpi_text: "800".to_owned(),
            sens_text: "0.35".to_owned(),
            fov_text: "103".to_owned(),
            game: 0,
            error: None,
            notice: None,
            dpi: 800.0,
            fov: 103.0,
            base_cm: 0.0,
            round: 0,
            lo: 0.5,
            hi: 1.5,
            order: [0.5, 1.5],
            slot: 0,
            results: [None, None],
            test: None,
            last_mouse: None,
            measure_ref: 0,
            measures: Vec::new(),
            measuring: false,
            measure_counts: Vec2::ZERO,
            measure_error: None,
            mice: mouse_info::detect(),
        }
    }

    /// Le dernier choix de jeu (index `GAMES.len()`) signifie « je ne connais pas ma sensibilité ».
    fn unknown_game(&self) -> bool {
        self.game == GAMES.len()
    }

    fn measured_dpi(&self) -> Option<f32> {
        if self.measures.is_empty() {
            None
        } else {
            Some(self.measures.iter().sum::<f32>() / self.measures.len() as f32)
        }
    }

    fn cm(&self, mult: f32) -> f32 {
        self.base_cm / mult
    }

    fn final_mult(&self) -> f32 {
        (self.lo + self.hi) / 2.0
    }

    fn grab(&mut self, on: bool) {
        set_cursor_grab(on);
        show_mouse(!on);
        self.last_mouse = None;
    }

    /// Avec le curseur capturé et `high_dpi`, macroquad cumule les comptes bruts (WM_INPUT).
    fn mouse_counts(&mut self) -> Vec2 {
        let (x, y) = mouse_position();
        let pos = vec2(x, y) * miniquad::window::dpi_scale();
        let delta = self.last_mouse.map_or(Vec2::ZERO, |last| pos - last);
        self.last_mouse = Some(pos);
        delta
    }

    fn new_round(&mut self) {
        self.order = if gen_range(0.0f32, 1.0) < 0.5 {
            [self.lo, self.hi]
        } else {
            [self.hi, self.lo]
        };
        self.slot = 0;
        self.results = [None, None];
        self.screen = Screen::Intro;
    }

    fn start(&mut self) {
        let parse = |s: &str| s.parse::<f32>().ok().filter(|v| v.is_finite() && *v > 0.0);
        let sens = if self.unknown_game() { Some(1.0) } else { parse(&self.sens_text) };
        let (Some(dpi), Some(sens), Some(fov)) = (parse(&self.dpi_text), sens, parse(&self.fov_text)) else {
            self.error = Some("Valeurs invalides : vérifiez le DPI, la sensibilité et le FOV.".to_owned());
            return;
        };
        if !(100.0..=32000.0).contains(&dpi) {
            self.error = Some("Le DPI doit être compris entre 100 et 32000.".to_owned());
            return;
        }
        if !(60.0..=130.0).contains(&fov) {
            self.error = Some("Le FOV horizontal doit être compris entre 60 et 130.".to_owned());
            return;
        }
        let cm = match GAMES.get(self.game) {
            Some(game) => games::cm_per_360(game.yaw, sens, dpi),
            None => games::DEFAULT_CM_PER_360,
        };
        if !(5.0..=200.0).contains(&cm) {
            self.error = Some(format!("{cm:.1} cm/360° semble irréaliste : vérifiez vos valeurs."));
            return;
        }
        self.error = None;
        self.notice = None;
        self.dpi = dpi;
        self.fov = fov;
        self.base_cm = cm;
        self.round = 0;
        self.lo = 0.5;
        self.hi = 1.5;
        self.new_round();
    }

    fn choose(&mut self, slot: usize) {
        let chosen = self.order[slot];
        let mid = (self.lo + self.hi) / 2.0;
        if chosen == self.lo {
            self.hi = mid;
        } else {
            self.lo = mid;
        }
        self.round += 1;
        if self.round >= ROUNDS {
            self.screen = Screen::Final;
        } else {
            self.new_round();
        }
    }

    fn save(&mut self) {
        let cm = self.cm(self.final_mult());
        let mut out = format!(
            "Aim Calibration — résultat\n\nSensibilité idéale : {cm:.2} cm/360° ({:.2} pouces/360°)\nDPI : {}\n\n",
            cm / 2.54,
            self.dpi
        );
        for game in GAMES {
            let sens = games::sens_for(game.yaw, cm, self.dpi);
            out += &format!("{:<30} {:.3}   (eDPI {:.0})\n", game.name, sens, sens * self.dpi);
        }
        self.notice = Some(match std::fs::write(RESULTS_FILE, out) {
            Ok(()) => format!("Résultat sauvegardé dans {RESULTS_FILE}"),
            Err(e) => format!("Échec de la sauvegarde : {e}"),
        });
    }

    /// Renvoie `true` pour quitter l'application.
    fn update(&mut self) -> bool {
        let mut chars = Vec::new();
        while let Some(c) = get_char_pressed() {
            chars.push(c.to_ascii_lowercase());
        }
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let escape = is_key_pressed(KeyCode::Escape);

        match self.screen {
            Screen::Setup => {
                if escape {
                    return true;
                }
                if is_key_pressed(KeyCode::F1) || chars.contains(&'h') {
                    self.screen = Screen::Help;
                    return false;
                }
                if chars.contains(&'m') {
                    self.measure_error = None;
                    self.screen = Screen::DpiMeasure;
                    return false;
                }
                if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::Tab) {
                    self.field = (self.field + 1) % 4;
                }
                if is_key_pressed(KeyCode::Up) {
                    self.field = (self.field + 3) % 4;
                }
                if self.field == 1 {
                    let choices = GAMES.len() + 1;
                    if is_key_pressed(KeyCode::Left) {
                        self.game = (self.game + choices - 1) % choices;
                    }
                    if is_key_pressed(KeyCode::Right) {
                        self.game = (self.game + 1) % choices;
                    }
                }
                let field = self.field;
                let unknown = self.unknown_game();
                let text = match field {
                    0 => Some(&mut self.dpi_text),
                    2 if !unknown => Some(&mut self.sens_text),
                    3 => Some(&mut self.fov_text),
                    _ => None,
                };
                if let Some(text) = text {
                    for &c in &chars {
                        if text.len() >= 8 {
                            break;
                        }
                        if c.is_ascii_digit() {
                            text.push(c);
                        } else if (c == '.' || c == ',') && field != 0 && !text.contains('.') {
                            text.push('.');
                        }
                    }
                    if is_key_pressed(KeyCode::Backspace) {
                        text.pop();
                    }
                }
                if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                    self.start();
                }
            }
            Screen::Intro => {
                if escape {
                    self.screen = Screen::Setup;
                } else if clicked {
                    let cm = self.cm(self.order[self.slot]);
                    self.test = Some(AimTest::new(games::deg_per_count(cm, self.dpi)));
                    self.grab(true);
                    self.screen = Screen::Playing;
                }
            }
            Screen::Playing => {
                if escape {
                    self.grab(false);
                    self.screen = Screen::Paused;
                    return false;
                }
                let counts = self.mouse_counts();
                let dt = get_frame_time().min(0.1);
                let Some(test) = self.test.as_mut() else {
                    return false;
                };
                test.update(counts, clicked, dt);
                if test.phase == Phase::Done {
                    self.results[self.slot] = Some(test.stats.clone());
                    self.test = None;
                    self.grab(false);
                    if self.slot == 0 {
                        self.slot = 1;
                        self.screen = Screen::Intro;
                    } else {
                        self.screen = Screen::RoundResult;
                    }
                }
            }
            Screen::Paused => {
                if clicked {
                    self.grab(true);
                    self.screen = Screen::Playing;
                } else if chars.contains(&'q') {
                    self.test = None;
                    self.screen = Screen::Setup;
                }
            }
            Screen::RoundResult => {
                if chars.contains(&'a') || is_key_pressed(KeyCode::Left) {
                    self.choose(0);
                } else if chars.contains(&'b') || is_key_pressed(KeyCode::Right) {
                    self.choose(1);
                } else if chars.contains(&'r') {
                    self.new_round();
                } else if escape {
                    self.screen = Screen::Setup;
                }
            }
            Screen::Final => {
                if chars.contains(&'s') {
                    self.save();
                } else if chars.contains(&'r') {
                    if self.unknown_game() {
                        self.game = 0;
                    }
                    let cm = self.cm(self.final_mult());
                    let sens = games::sens_for(GAMES[self.game].yaw, cm, self.dpi);
                    self.sens_text = format!("{sens:.3}");
                    self.notice = None;
                    self.screen = Screen::Setup;
                } else if escape {
                    self.screen = Screen::Setup;
                }
            }
            Screen::Help => {
                if escape || clicked || is_key_pressed(KeyCode::F1) || is_key_pressed(KeyCode::Enter) {
                    self.screen = Screen::Setup;
                }
            }
            Screen::DpiMeasure => self.update_dpi_measure(&chars, escape),
        }
        false
    }

    fn update_dpi_measure(&mut self, chars: &[char], escape: bool) {
        if self.measuring {
            let counts = self.mouse_counts();
            self.measure_counts += counts;
            if !is_mouse_button_down(MouseButton::Left) {
                self.measuring = false;
                self.grab(false);
                let counts = self.measure_counts.length();
                if counts < 50.0 {
                    self.measure_error = Some("Mouvement trop court : glissez sur toute la longueur.".to_owned());
                } else {
                    let cm = MEASURE_REFS[self.measure_ref].1;
                    self.measures.push(counts / (cm / 2.54));
                    if self.measures.len() > 3 {
                        self.measures.remove(0);
                    }
                    self.measure_error = None;
                }
            }
            return;
        }

        if escape {
            self.screen = Screen::Setup;
        } else if is_key_pressed(KeyCode::Left) {
            self.measure_ref = (self.measure_ref + MEASURE_REFS.len() - 1) % MEASURE_REFS.len();
        } else if is_key_pressed(KeyCode::Right) {
            self.measure_ref = (self.measure_ref + 1) % MEASURE_REFS.len();
        } else if chars.contains(&'r') {
            self.measures.clear();
        } else if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            if let Some(dpi) = self.measured_dpi() {
                self.dpi_text = format!("{:.0}", games::likely_dpi(dpi));
                self.field = 1;
                self.screen = Screen::Setup;
            }
        } else if is_mouse_button_pressed(MouseButton::Left) {
            self.measuring = true;
            self.measure_counts = Vec2::ZERO;
            self.grab(true);
        }
    }

    fn draw(&self) {
        match (&self.test, self.screen) {
            (Some(test), Screen::Playing | Screen::Paused) => {
                set_camera(&aim::camera(test.yaw, test.pitch, self.fov));
                aim::draw_world();
                aim::draw_target(test);
            }
            _ => {
                let yaw = (get_time() * 4.0 % 360.0) as f32;
                set_camera(&aim::camera(yaw, 4.0, self.fov.clamp(60.0, 130.0)));
                aim::draw_world();
            }
        }
        set_default_camera();

        match self.screen {
            Screen::Playing => self.draw_hud(),
            Screen::Paused => {
                dim(0.6);
                self.ui.center("PAUSE", 460.0, 80.0, WHITE_SOFT);
                self.ui.center("Clic : reprendre  ·  Q : abandonner et revenir au menu", 540.0, 30.0, GREY);
            }
            Screen::Setup => self.draw_setup(),
            Screen::Intro => self.draw_intro(),
            Screen::RoundResult => self.draw_round_result(),
            Screen::Final => self.draw_final(),
            Screen::Help => self.draw_help(),
            Screen::DpiMeasure => self.draw_dpi_measure(),
        }
    }

    fn draw_hud(&self) {
        aim::draw_crosshair();
        let Some(test) = &self.test else { return };
        let label = format!("Manche {}/{}  ·  Sensibilité {}", self.round + 1, ROUNDS, slot_name(self.slot));
        self.ui.text(&label, 30.0, 50.0, 28.0, GREY);
        self.ui.center(&test.status(), 70.0, 34.0, WHITE_SOFT);
        self.ui.text("Échap : pause", 30.0, 1050.0, 22.0, GREY);
    }

    fn draw_setup(&self) {
        dim(0.75);
        let ui = &self.ui;
        ui.center("AIM CALIBRATION", 200.0, 90.0, ACCENT);
        ui.center("Trouvez votre sensibilité idéale pour vos FPS", 260.0, 32.0, GREY);

        let game = GAMES.get(self.game);
        let cursor = if (get_time() * 2.0) as i64 % 2 == 0 { "_" } else { " " };
        let fields = [
            ("DPI de la souris", self.dpi_text.clone()),
            (
                "Jeu de référence",
                format!("<  {}  >", game.map_or("Je ne sais pas", |g| g.name)),
            ),
            (
                "Sensibilité actuelle dans ce jeu",
                if game.is_some() { self.sens_text.clone() } else { "—".to_owned() },
            ),
            ("FOV horizontal (Valorant : 103)", self.fov_text.clone()),
        ];
        let left = screen_width() / 2.0 - 520.0 * Ui::scale();
        let right = screen_width() / 2.0 + 40.0 * Ui::scale();
        for (i, (label, value)) in fields.iter().enumerate() {
            let y = 380.0 + i as f32 * 70.0;
            let selected = i == self.field;
            let color = if selected { WHITE_SOFT } else { GREY };
            if selected {
                draw_rectangle(
                    left - 20.0 * Ui::scale(),
                    (y - 45.0) * Ui::scale(),
                    1080.0 * Ui::scale(),
                    62.0 * Ui::scale(),
                    Color::new(0.0, 1.0, 0.78, 0.08),
                );
            }
            ui.text(label, left, y, 32.0, color);
            let editable = i == 0 || i == 3 || (i == 2 && game.is_some());
            let value = if selected && editable { format!("{value}{cursor}") } else { value.clone() };
            ui.text(&value, right, y, 32.0, if selected { ACCENT } else { color });
        }

        let hint = match (self.field, game) {
            (0, _) => "La précision de votre souris (souvent 400, 800 ou 1600). Vous ne la connaissez pas ? Appuyez sur M pour la détecter et la mesurer.".to_owned(),
            (1, _) => "Le jeu dont vous connaissez la sensibilité. Aucune idée ? Choisissez « Je ne sais pas » (← →).".to_owned(),
            (2, Some(g)) => format!("Dans {} : {}", g.name, g.settings),
            (2, None) => "Pas besoin : le test partira d'une valeur moyenne, puis s'adaptera à vous.".to_owned(),
            _ => "L'angle de vue du jeu. Dans le doute, laissez 103 (valeur fixe de Valorant).".to_owned(),
        };
        ui.center(&hint, 670.0, 26.0, WHITE_SOFT);

        let preview = match (self.dpi_text.parse::<f32>(), self.sens_text.parse::<f32>(), game) {
            (Ok(dpi), _, None) if dpi > 0.0 => format!(
                "Départ à {:.0} cm pour un tour complet (360°), une valeur moyenne chez les joueurs FPS",
                games::DEFAULT_CM_PER_360
            ),
            (Ok(dpi), Ok(sens), Some(g)) if dpi > 0.0 && sens > 0.0 => {
                let cm = games::cm_per_360(g.yaw, sens, dpi);
                format!("= {cm:.1} cm pour un tour complet (360°)  ·  eDPI {:.0}", dpi * sens)
            }
            _ => String::new(),
        };
        ui.center(&preview, 720.0, 30.0, GOLD);
        if let Some(err) = &self.error {
            ui.center(err, 765.0, 28.0, RED);
        }

        let help = [
            format!(
                "{ROUNDS} manches. À chaque manche, deux sensibilités sont testées à l'aveugle (A et B) :"
            ),
            format!("{FLICK_TARGETS} cibles à éliminer en flick, puis {TRACK_DURATION:.0} s de tracking. Choisissez la meilleure, l'écart se resserre."),
        ];
        ui.center(&help[0], 820.0, 26.0, GREY);
        ui.center(&help[1], 856.0, 26.0, GREY);
        ui.center(
            "↑ ↓ : champ  ·  ← → : jeu  ·  Entrée : commencer  ·  H / F1 : aide  ·  M : mesurer le DPI  ·  F11 : plein écran  ·  Échap : quitter",
            1020.0,
            24.0,
            GREY,
        );
    }

    fn draw_intro(&self) {
        dim(0.65);
        let ui = &self.ui;
        ui.center(&format!("Manche {} / {}", self.round + 1, ROUNDS), 380.0, 40.0, GREY);
        ui.center(&format!("Sensibilité {}", slot_name(self.slot)), 490.0, 100.0, ACCENT);
        ui.center(
            &format!("{FLICK_TARGETS} cibles rouges à éliminer, puis {TRACK_DURATION:.0} s à suivre la cible orange"),
            580.0,
            30.0,
            WHITE_SOFT,
        );
        ui.center("Cliquez pour commencer", 680.0, 36.0, GOLD);
        ui.center("Échap : retour au menu", 1020.0, 24.0, GREY);
    }

    fn draw_round_result(&self) {
        dim(0.75);
        let ui = &self.ui;
        ui.center(&format!("Manche {} / {} — résultats", self.round + 1, ROUNDS), 160.0, 50.0, WHITE_SOFT);

        let scores: Vec<f32> = self.results.iter().map(|r| r.as_ref().map_or(0.0, Stats::score)).collect();
        let best = if scores[0] >= scores[1] { 0 } else { 1 };
        for slot in 0..2 {
            let Some(stats) = &self.results[slot] else { continue };
            let cx = screen_width() / 2.0 + (slot as f32 * 2.0 - 1.0) * 360.0 * Ui::scale();
            let color = if slot == best { GOLD } else { WHITE_SOFT };
            ui.center_at(&format!("Sensibilité {}", slot_name(slot)), cx, 290.0, 56.0, color);
            ui.center_at(&format!("{:.1} cm/360°", self.cm(self.order[slot])), cx, 350.0, 32.0, ACCENT);
            let lines = [
                format!("Précision : {:.0} %", stats.accuracy() * 100.0),
                format!("Temps moyen par cible : {:.0} ms", stats.avg_flick_ms()),
                format!("Tracking : {:.0} % du temps sur la cible", stats.tracking_pct()),
            ];
            for (i, line) in lines.iter().enumerate() {
                ui.center_at(line, cx, 430.0 + i as f32 * 46.0, 28.0, GREY);
            }
            ui.center_at(&format!("Score : {:.0}", stats.score()), cx, 600.0, 44.0, color);
            if slot == best {
                ui.center_at("Meilleur score", cx, 650.0, 26.0, GOLD);
            }
        }

        ui.center("Laquelle vous a semblé la plus naturelle et contrôlée ?", 780.0, 34.0, WHITE_SOFT);
        ui.center("Le score est indicatif : fiez-vous aussi à votre ressenti.", 825.0, 26.0, GREY);
        ui.center(
            "A ou ← : garder A  ·  B ou → : garder B  ·  R : rejouer la manche  ·  Échap : menu",
            900.0,
            28.0,
            GOLD,
        );
    }

    fn draw_final(&self) {
        dim(0.8);
        let ui = &self.ui;
        let mult = self.final_mult();
        let cm = self.cm(mult);
        ui.center("Votre sensibilité idéale", 150.0, 44.0, GREY);
        ui.center(&format!("{cm:.1} cm / 360°"), 250.0, 100.0, ACCENT);
        ui.center(
            &format!("{:.1} pouces / 360°  ·  à {:.0} DPI", cm / 2.54, self.dpi),
            310.0,
            30.0,
            WHITE_SOFT,
        );

        let left = screen_width() / 2.0 - 400.0 * Ui::scale();
        let right = screen_width() / 2.0 + 180.0 * Ui::scale();
        for (i, game) in GAMES.iter().enumerate() {
            let y = 400.0 + i as f32 * 50.0;
            let sens = games::sens_for(game.yaw, cm, self.dpi);
            let color = if i == self.game { GOLD } else { WHITE_SOFT };
            ui.text(game.name, left, y, 32.0, color);
            ui.text(&format!("{sens:.3}"), right, y, 32.0, color);
            ui.text(&format!("eDPI {:.0}", sens * self.dpi), right + 140.0 * Ui::scale(), y, 24.0, GREY);
        }

        let mut y = 400.0 + GAMES.len() as f32 * 50.0 + 30.0;
        if !(0.53..=1.47).contains(&mult) {
            ui.center(
                "Résultat en bord de plage : appuyez sur R pour relancer à partir de cette valeur et aller plus loin.",
                y,
                26.0,
                RED,
            );
            y += 40.0;
        }
        ui.center(
            "League of Legends n'est pas un FPS : seul le curseur compte. Gardez la vitesse du pointeur Windows à 10/20,",
            y + 10.0,
            24.0,
            GREY,
        );
        ui.center(
            "désactivez « Améliorer la précision du pointeur », et réglez la vitesse en jeu à votre confort.",
            y + 44.0,
            24.0,
            GREY,
        );
        if let Some(notice) = &self.notice {
            ui.center(notice, 960.0, 26.0, ACCENT);
        }
        ui.center(
            "S : sauvegarder  ·  R : affiner à partir de ce résultat  ·  Échap : menu",
            1020.0,
            28.0,
            GOLD,
        );
    }
}

impl App {
    fn draw_help(&self) {
        dim(0.88);
        let ui = &self.ui;
        ui.center("Aide — les bases", 120.0, 60.0, ACCENT);

        let sections: &[(&str, &[&str])] = &[
            (
                "La sensibilité en cm/360°",
                &[
                    "C'est la distance que parcourt votre souris sur le tapis pour faire un tour complet dans le jeu.",
                    "Elle se transpose d'un jeu à l'autre : seul le chiffre affiché dans les options change.",
                    "Repères : 25 à 35 cm pour Apex ou Overwatch, 40 à 55 cm pour Valorant ou CS2.",
                ],
            ),
            (
                "Le DPI",
                &[
                    "La précision de la souris : le nombre de points détectés par pouce (2,54 cm) de déplacement.",
                    "Il se règle dans le logiciel de la souris (Logitech G HUB, Razer Synapse, SteelSeries GG…)",
                    "ou avec un bouton sous la souris. Inconnu ? Mesurez-le dans le menu avec la touche M.",
                ],
            ),
            (
                "La sensibilité en jeu et l'eDPI",
                &[
                    "La sensibilité est le réglage dans les options du jeu. L'eDPI = DPI × sensibilité :",
                    "deux joueurs à 400 DPI / 0,8 et 800 DPI / 0,4 ont exactement la même vitesse.",
                ],
            ),
            (
                "Avant de commencer",
                &[
                    "Windows : Paramètres > Bluetooth et appareils > Souris > Paramètres de souris supplémentaires",
                    "> Options du pointeur : décochez « Améliorer la précision du pointeur » (c'est une accélération).",
                    "Jouez comme en vrai : même posture, même tapis. Refaites le test un autre jour pour confirmer.",
                ],
            ),
        ];

        let left = screen_width() / 2.0 - 680.0 * Ui::scale();
        let mut y = 220.0;
        for (title, lines) in sections {
            ui.text(title, left, y, 34.0, GOLD);
            y += 44.0;
            for line in *lines {
                ui.text(line, left + 20.0 * Ui::scale(), y, 26.0, WHITE_SOFT);
                y += 36.0;
            }
            y += 26.0;
        }
        ui.center("Échap, Entrée ou clic : retour", 1030.0, 26.0, GREY);
    }

    fn draw_dpi_measure(&self) {
        dim(0.88);
        let ui = &self.ui;
        let (ref_name, _) = MEASURE_REFS[self.measure_ref];
        ui.center("Mesurer le DPI de votre souris", 140.0, 60.0, ACCENT);

        let left = screen_width() / 2.0 - 640.0 * Ui::scale();
        ui.text("1.  Objet de longueur connue :", left, 250.0, 30.0, WHITE_SOFT);
        ui.text(&format!("<  {ref_name}  >   (← → pour changer)"), left + 470.0 * Ui::scale(), 250.0, 30.0, GOLD);
        let steps = [
            "2.  Posez-le à plat sur le tapis, et alignez le bord gauche de la souris avec l'extrémité gauche de l'objet.",
            "3.  Maintenez le clic gauche, glissez la souris en ligne droite jusqu'à ce que son bord gauche",
            "      atteigne l'autre extrémité de l'objet, puis relâchez.",
            "4.  Recommencez 2 ou 3 fois : la moyenne est plus fiable.",
        ];
        for (i, step) in steps.iter().enumerate() {
            ui.text(step, left, 310.0 + i as f32 * 50.0, 28.0, WHITE_SOFT);
        }

        if self.measuring {
            ui.center(&format!("Mesure en cours…  {:.0} points", self.measure_counts.length()), 620.0, 44.0, ACCENT);
        } else {
            ui.center("Maintenez le clic gauche pour mesurer", 620.0, 36.0, GOLD);
        }
        if let Some(err) = &self.measure_error {
            ui.center(err, 670.0, 28.0, RED);
        }

        if let Some(avg) = self.measured_dpi() {
            let list: Vec<String> = self.measures.iter().map(|m| format!("{m:.0}")).collect();
            ui.center(&format!("Mesures : {}", list.join("  ·  ")), 740.0, 28.0, GREY);
            ui.center(
                &format!("Moyenne {avg:.0} DPI  →  votre souris est très probablement à {:.0} DPI", games::likely_dpi(avg)),
                800.0,
                34.0,
                WHITE_SOFT,
            );
        }

        let main_mouse = self.mice.iter().find(|m| m.brand.is_some()).or(self.mice.first());
        match main_mouse {
            Some(mouse) => {
                let others = match self.mice.len() - 1 {
                    0 => String::new(),
                    n => format!("   (+{n} autre{} détectée{})", if n > 1 { "s" } else { "" }, if n > 1 { "s" } else { "" }),
                };
                ui.center(&format!("Souris détectée : {}{others}", mouse.name), 880.0, 30.0, ACCENT);
                let advice = mouse.brand.map_or(
                    "Marque sans logiciel connu : le DPI change souvent avec un bouton près de la molette. Fiez-vous à la mesure.",
                    |b| b.advice,
                );
                ui.center(advice, 925.0, 26.0, WHITE_SOFT);
            }
            None => ui.center(
                "Le DPI est aussi indiqué dans le logiciel de la souris ou sur sa fiche produit.",
                920.0,
                24.0,
                GREY,
            ),
        }
        ui.center("Entrée : utiliser cette valeur  ·  R : effacer les mesures  ·  Échap : retour", 1020.0, 28.0, GOLD);
    }
}

fn slot_name(slot: usize) -> &'static str {
    if slot == 0 { "A" } else { "B" }
}

fn dim(alpha: f32) {
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.03, 0.04, 0.06, alpha));
}

#[macroquad::main(window_conf)]
async fn main() {
    srand((miniquad::date::now() * 1000.0) as u64);
    let mut app = App::new();
    let mut fullscreen = true;
    loop {
        if is_key_pressed(KeyCode::F11) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }
        if app.update() {
            break;
        }
        app.draw();
        next_frame().await;
    }
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod aim;
mod games;
mod mouse_info;
mod persist;
mod sfx;

use aim::{
    AimTest, Crosshair, DEFAULT_DURATION, DURATIONS, FLICK_TARGETS, LIFETIMES, MOTIONS, NORMAL, PRESETS, Phase,
    SIZES, SPEEDS, Settings, Stats, TRACK_DURATION, TestKind, XHAIR_COLORS, XHAIR_GAPS, XHAIR_LENGTHS, XHAIR_THICK,
    ZONES,
};
use persist::Entry;
use sfx::Sfx;
use games::GAMES;
use mouse_info::MouseInfo;
use macroquad::miniquad;
use macroquad::prelude::*;
use macroquad::rand::{gen_range, srand};

const ROUNDS: usize = 6;
const RESULTS_FILE: &str = "resultats_sensibilite.txt";
const RECORDS_FILE: &str = "records_entrainement.txt";

const MENU: &[(&str, &str)] = &[
    ("Trouver ma sensibilité", "6 manches de comparaison à l'aveugle pour trouver votre réglage idéal"),
    ("Entraînement", "Flick, tracking, micro-ajustements, switching, réflexes et routine"),
    ("Historique", "L'évolution de vos scores"),
    ("Viseur", "Couleur, écart, longueur, épaisseur et point central"),
    ("Aide", "DPI, cm/360°, eDPI : tout comprendre en une minute"),
    ("Quitter", "À bientôt !"),
];

const TRAIN_MENU: &[(&str, &str)] = &[
    ("Flick", "De grands mouvements pour éliminer des cibles éloignées"),
    ("Tracking", "Gardez le viseur sur une cible qui se déplace"),
    ("Micro-ajustements", "De toutes petites corrections autour du viseur"),
    ("Switching", "Alternez rapidement entre deux cibles"),
    ("Réflexes", "Tirez dès que la cible apparaît"),
    ("Routine du jour", "10 minutes guidées : 5 exercices de 2 minutes"),
    ("Retour", "Revenir au menu principal"),
];

const ROUTINE: &[TestKind] = &[TestKind::Flick, TestKind::Tracking, TestKind::Micro, TestKind::Switching, TestKind::Reflex];

/// Records par mode (5), difficulté prédéfinie et durée.
type Records = [[[f32; 4]; 4]; 5];

#[derive(Clone, Copy, PartialEq)]
enum Row {
    Preset,
    Duration,
    Size,
    Speed,
    Motion,
    Lifetime,
    Zone,
    Vertical,
    Shape,
}

fn training_rows(kind: TestKind) -> &'static [Row] {
    match kind {
        TestKind::Tracking => &[Row::Preset, Row::Duration, Row::Size, Row::Speed, Row::Vertical, Row::Zone],
        TestKind::Micro => &[Row::Preset, Row::Duration, Row::Size, Row::Zone, Row::Shape],
        TestKind::Switching => &[Row::Preset, Row::Duration, Row::Size, Row::Motion, Row::Zone, Row::Shape],
        TestKind::Reflex => &[Row::Preset, Row::Duration, Row::Size, Row::Zone, Row::Shape],
        _ => &[Row::Preset, Row::Duration, Row::Size, Row::Motion, Row::Lifetime, Row::Zone, Row::Shape],
    }
}

fn cycle(value: usize, len: usize, delta: i32) -> usize {
    (value as i32 + delta).rem_euclid(len as i32) as usize
}

fn row_label(row: Row) -> &'static str {
    match row {
        Row::Preset => "Difficulté",
        Row::Duration => "Durée",
        Row::Size => "Taille des cibles",
        Row::Speed => "Vitesse de la cible",
        Row::Motion => "Cibles mobiles",
        Row::Lifetime => "Durée de vie des cibles",
        Row::Zone => "Zone d'apparition",
        Row::Vertical => "Mouvements verticaux",
        Row::Shape => "Forme des cibles",
    }
}

fn row_value(row: Row, settings: &Settings) -> &'static str {
    match row {
        Row::Preset => settings.matching_preset().map_or("Personnalisé", |p| PRESETS[p]),
        Row::Duration => DURATIONS[settings.duration].0,
        Row::Size => SIZES[settings.size].0,
        Row::Speed => SPEEDS[settings.speed].0,
        Row::Motion => MOTIONS[settings.motion].0,
        Row::Lifetime => LIFETIMES[settings.lifetime].0,
        Row::Zone => ZONES[settings.zone].0,
        Row::Vertical => {
            if settings.vertical {
                "Oui"
            } else {
                "Non"
            }
        }
        Row::Shape => {
            if settings.humanoid {
                "Humanoïde"
            } else {
                "Sphère"
            }
        }
    }
}

fn adjust_row(row: Row, settings: &mut Settings, delta: i32) {
    match row {
        Row::Preset => {
            let preset = settings.matching_preset().map_or(NORMAL, |p| cycle(p, PRESETS.len(), delta));
            *settings = Settings::preset(preset, settings.duration);
        }
        Row::Duration => settings.duration = cycle(settings.duration, DURATIONS.len(), delta),
        Row::Size => settings.size = cycle(settings.size, SIZES.len(), delta),
        Row::Speed => settings.speed = cycle(settings.speed, SPEEDS.len(), delta),
        Row::Motion => settings.motion = cycle(settings.motion, MOTIONS.len(), delta),
        Row::Lifetime => settings.lifetime = cycle(settings.lifetime, LIFETIMES.len(), delta),
        Row::Zone => settings.zone = cycle(settings.zone, ZONES.len(), delta),
        Row::Vertical => settings.vertical = !settings.vertical,
        Row::Shape => settings.humanoid = !settings.humanoid,
    }
}

fn settings_summary(settings: &Settings) -> String {
    let preset = settings.matching_preset().map_or("Personnalisé", |p| PRESETS[p]);
    format!("{preset}  ·  {}", DURATIONS[settings.duration].0)
}

fn record_index(kind: TestKind) -> Option<usize> {
    match kind {
        TestKind::Flick => Some(0),
        TestKind::Tracking => Some(1),
        TestKind::Micro => Some(2),
        TestKind::Switching => Some(3),
        TestKind::Reflex => Some(4),
        TestKind::Calibration => None,
    }
}

fn load_records() -> Records {
    let mut records = [[[0.0; 4]; 4]; 5];
    let Ok(text) = std::fs::read_to_string(RECORDS_FILE) else {
        return records;
    };
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let [mode, preset, duration, score] = parts[..]
            && let (Ok(mode), Ok(preset), Ok(duration), Ok(score)) =
                (mode.parse::<usize>(), preset.parse::<usize>(), duration.parse::<usize>(), score.parse())
            && mode < 5
            && preset < PRESETS.len()
            && duration < DURATIONS.len()
        {
            records[mode][preset][duration] = score;
        }
    }
    records
}

fn mode_name(kind: TestKind) -> &'static str {
    match kind {
        TestKind::Calibration => "Trouver ma sensibilité",
        TestKind::Flick => "Flick",
        TestKind::Tracking => "Tracking",
        TestKind::Micro => "Micro-ajustements",
        TestKind::Switching => "Switching",
        TestKind::Reflex => "Réflexes",
    }
}

fn routine_settings() -> Settings {
    Settings::preset(NORMAL, DURATIONS.len() - 1)
}

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
    MainMenu,
    TrainMenu,
    Setup,
    TrainingIntro,
    TrainingResult,
    Crosshair,
    History,
    RoutineBrief,
    RoutineSummary,
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
    mode: TestKind,
    menu_sel: usize,
    train_sel: usize,
    help_return: Screen,
    crosshair: Crosshair,
    xhair_row: usize,
    history: Vec<Entry>,
    history_mode: usize,
    in_routine: bool,
    routine_i: usize,
    routine_scores: Vec<(TestKind, f32)>,
    sfx: Sfx,
    train: Settings,
    train_row: usize,
    result_settings: Settings,
    train_cm: f32,
    training_result: Option<Stats>,
    records: Records,
    new_record: bool,
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
    fn new(sfx: Sfx) -> Self {
        let profile = persist::load_profile();
        Self {
            ui: Ui::load(),
            screen: Screen::MainMenu,
            mode: TestKind::Calibration,
            menu_sel: 0,
            train_sel: 0,
            help_return: Screen::MainMenu,
            crosshair: profile.crosshair,
            xhair_row: 0,
            history: persist::load_history(),
            history_mode: 0,
            in_routine: false,
            routine_i: 0,
            routine_scores: Vec::new(),
            sfx,
            train: profile.train,
            train_row: 0,
            result_settings: Settings::preset(NORMAL, DEFAULT_DURATION),
            train_cm: games::DEFAULT_CM_PER_360,
            training_result: None,
            records: load_records(),
            new_record: false,
            field: 0,
            dpi_text: profile.dpi,
            sens_text: profile.sens,
            fov_text: profile.fov,
            game: profile.game.min(GAMES.len()),
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

    /// Valide les champs de réglage ; renvoie la sensibilité en cm/360°.
    fn read_settings(&mut self) -> Option<f32> {
        let parse = |s: &str| s.parse::<f32>().ok().filter(|v| v.is_finite() && *v > 0.0);
        let sens = if self.unknown_game() { Some(1.0) } else { parse(&self.sens_text) };
        let (Some(dpi), Some(sens), Some(fov)) = (parse(&self.dpi_text), sens, parse(&self.fov_text)) else {
            self.error = Some("Valeurs invalides : vérifiez le DPI, la sensibilité et le FOV.".to_owned());
            return None;
        };
        if !(100.0..=32000.0).contains(&dpi) {
            self.error = Some("Le DPI doit être compris entre 100 et 32000.".to_owned());
            return None;
        }
        if !(60.0..=130.0).contains(&fov) {
            self.error = Some("Le FOV horizontal doit être compris entre 60 et 130.".to_owned());
            return None;
        }
        let cm = match GAMES.get(self.game) {
            Some(game) => games::cm_per_360(game.yaw, sens, dpi),
            None => games::DEFAULT_CM_PER_360,
        };
        if !(5.0..=200.0).contains(&cm) {
            self.error = Some(format!("{cm:.1} cm/360° semble irréaliste : vérifiez vos valeurs."));
            return None;
        }
        self.error = None;
        self.notice = None;
        self.dpi = dpi;
        self.fov = fov;
        Some(cm)
    }

    fn start(&mut self) {
        let Some(cm) = self.read_settings() else { return };
        match self.mode {
            TestKind::Calibration => {
                self.base_cm = cm;
                self.round = 0;
                self.lo = 0.5;
                self.hi = 1.5;
                self.new_round();
            }
            _ => {
                self.train_cm = cm;
                if self.in_routine {
                    self.routine_i = 0;
                    self.routine_scores.clear();
                    self.mode = ROUTINE[0];
                    self.train = routine_settings();
                    self.screen = Screen::RoutineBrief;
                } else {
                    self.screen = Screen::TrainingIntro;
                }
            }
        }
        self.save_profile();
    }

    fn save_profile(&self) {
        persist::save_profile(&persist::Profile {
            dpi: self.dpi_text.clone(),
            sens: self.sens_text.clone(),
            fov: self.fov_text.clone(),
            game: self.game,
            train: self.train,
            crosshair: self.crosshair,
        });
    }

    /// Met à jour la sélection d'un menu. Renvoie `(sélection, validé)`.
    fn menu_action(&self, len: usize, mut sel: usize, clicked: bool) -> (usize, bool) {
        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::Tab) {
            sel = cycle(sel, len, 1);
        }
        if is_key_pressed(KeyCode::Up) {
            sel = cycle(sel, len, -1);
        }
        let (mx, my) = mouse_position();
        let hovered = (0..len).find(|&i| menu_rect(i, len).contains(vec2(mx, my)));
        if let Some(i) = hovered {
            sel = i;
        }
        let activate = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) || (clicked && hovered.is_some());
        (sel, activate)
    }

    fn update_crosshair(&mut self, escape: bool) {
        const ROWS: usize = 5;
        if escape || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            self.save_profile();
            self.screen = Screen::MainMenu;
            return;
        }
        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::Tab) {
            self.xhair_row = cycle(self.xhair_row, ROWS, 1);
        }
        if is_key_pressed(KeyCode::Up) {
            self.xhair_row = cycle(self.xhair_row, ROWS, -1);
        }
        let delta = if is_key_pressed(KeyCode::Right) { 1 } else if is_key_pressed(KeyCode::Left) { -1 } else { 0 };
        if delta != 0 {
            let crosshair = &mut self.crosshair;
            match self.xhair_row {
                0 => crosshair.color = cycle(crosshair.color, XHAIR_COLORS.len(), delta),
                1 => crosshair.gap = cycle(crosshair.gap, XHAIR_GAPS.len(), delta),
                2 => crosshair.length = cycle(crosshair.length, XHAIR_LENGTHS.len(), delta),
                3 => crosshair.thick = cycle(crosshair.thick, XHAIR_THICK.len(), delta),
                _ => crosshair.dot = !crosshair.dot,
            }
            self.save_profile();
        }
    }

    fn open_help(&mut self) {
        self.help_return = self.screen;
        self.screen = Screen::Help;
    }

    fn activate_menu(&mut self, item: usize) -> bool {
        self.error = None;
        match item {
            0 => {
                self.mode = TestKind::Calibration;
                self.in_routine = false;
                self.screen = Screen::Setup;
            }
            1 => self.screen = Screen::TrainMenu,
            2 => self.screen = Screen::History,
            3 => self.screen = Screen::Crosshair,
            4 => self.open_help(),
            _ => return true,
        }
        false
    }

    fn activate_train(&mut self, item: usize) {
        const MODES: [TestKind; 5] = [TestKind::Flick, TestKind::Tracking, TestKind::Micro, TestKind::Switching, TestKind::Reflex];
        if let Some(mode) = MODES.get(item) {
            self.mode = *mode;
            self.in_routine = false;
            self.error = None;
            self.screen = Screen::Setup;
        } else if item == MODES.len() {
            self.in_routine = true;
            self.mode = TestKind::Flick;
            self.error = None;
            self.screen = Screen::Setup;
        } else {
            self.screen = Screen::MainMenu;
        }
    }

    /// Les records ne sont tenus que pour les difficultés prédéfinies.
    fn record_slot(&mut self, settings: &Settings) -> Option<&mut f32> {
        let mode = record_index(self.mode)?;
        let preset = settings.matching_preset()?;
        Some(&mut self.records[mode][preset][settings.duration])
    }

    fn record(&self, settings: &Settings) -> Option<f32> {
        let mode = record_index(self.mode)?;
        let preset = settings.matching_preset()?;
        Some(self.records[mode][preset][settings.duration])
    }

    fn finish_training(&mut self, stats: Stats) {
        let score = stats.training_score(self.mode);
        let settings = self.train;
        self.new_record = false;
        if let Some(best) = self.record_slot(&settings)
            && score > *best
        {
            *best = score;
            self.new_record = true;
        }
        if self.new_record {
            self.save_records();
        }
        self.result_settings = settings;
        self.training_result = Some(stats.clone());
        if let Some(mode) = record_index(self.mode) {
            let note = match self.mode {
                TestKind::Tracking => format!("suivi {:.0} %", stats.tracking_pct()),
                TestKind::Reflex => format!("réaction {:.0} ms", stats.avg_flick_ms()),
                _ => format!("précision {:.0} %", stats.accuracy() * 100.0),
            };
            let entry = Entry { time: miniquad::date::now(), mode, score: stats.training_score(self.mode), note };
            persist::append_history(&entry);
            self.history.push(entry);
        }
        if self.in_routine {
            self.routine_scores.push((self.mode, stats.training_score(self.mode)));
            self.routine_i += 1;
            if self.routine_i >= ROUTINE.len() {
                self.in_routine = false;
                self.screen = Screen::RoutineSummary;
            } else {
                self.mode = ROUTINE[self.routine_i];
                self.train = routine_settings();
                self.screen = Screen::RoutineBrief;
            }
        } else {
            self.screen = Screen::TrainingResult;
        }
    }

    fn save_records(&self) {
        let mut out = String::new();
        for (mode, presets) in self.records.iter().enumerate() {
            for (preset, durations) in presets.iter().enumerate() {
                for (duration, score) in durations.iter().enumerate() {
                    if *score > 0.0 {
                        out += &format!("{mode} {preset} {duration} {score}\n");
                    }
                }
            }
        }
        let _ = std::fs::write(RECORDS_FILE, out);
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
            if self.unknown_game() {
                self.game = 0;
            }
            let sens = games::sens_for(GAMES[self.game].yaw, self.cm(self.final_mult()), self.dpi);
            self.sens_text = format!("{sens:.3}");
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
            Screen::MainMenu => {
                if escape {
                    self.save_profile();
                    return true;
                }
                let (sel, go) = self.menu_action(MENU.len(), self.menu_sel, clicked);
                self.menu_sel = sel;
                if go {
                    return self.activate_menu(sel);
                }
                if is_key_pressed(KeyCode::F1) {
                    self.open_help();
                }
            }
            Screen::TrainMenu => {
                if escape {
                    self.screen = Screen::MainMenu;
                } else {
                    let (sel, go) = self.menu_action(TRAIN_MENU.len(), self.train_sel, clicked);
                    self.train_sel = sel;
                    if go {
                        self.activate_train(sel);
                    }
                }
            }
            Screen::Crosshair => self.update_crosshair(escape),
            Screen::History => {
                if escape || clicked {
                    self.screen = Screen::MainMenu;
                } else if is_key_pressed(KeyCode::Left) {
                    self.history_mode = cycle(self.history_mode, 5, -1);
                } else if is_key_pressed(KeyCode::Right) {
                    self.history_mode = cycle(self.history_mode, 5, 1);
                }
            }
            Screen::RoutineBrief => {
                if escape {
                    self.in_routine = false;
                    self.screen = Screen::MainMenu;
                } else if clicked || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                    let deg_per_count = games::deg_per_count(self.train_cm, self.dpi);
                    self.test = Some(AimTest::new(deg_per_count, self.mode, self.train));
                    self.grab(true);
                    self.screen = Screen::Playing;
                }
            }
            Screen::RoutineSummary => {
                if escape || clicked || is_key_pressed(KeyCode::Enter) {
                    self.screen = Screen::MainMenu;
                }
            }
            Screen::Setup => {
                if escape {
                    self.save_profile();
                    self.in_routine = false;
                    self.screen = Screen::MainMenu;
                    return false;
                }
                if is_key_pressed(KeyCode::F1) || chars.contains(&'h') {
                    self.open_help();
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
                    self.test = Some(AimTest::new(games::deg_per_count(cm, self.dpi), TestKind::Calibration, self.train));
                    self.grab(true);
                    self.screen = Screen::Playing;
                }
            }
            Screen::TrainingIntro => {
                let rows = training_rows(self.mode);
                self.train_row = self.train_row.min(rows.len() - 1);
                if escape {
                    self.screen = Screen::Setup;
                } else if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::Tab) {
                    self.train_row = cycle(self.train_row, rows.len(), 1);
                } else if is_key_pressed(KeyCode::Up) {
                    self.train_row = cycle(self.train_row, rows.len(), -1);
                } else if is_key_pressed(KeyCode::Left) {
                    adjust_row(rows[self.train_row], &mut self.train, -1);
                } else if is_key_pressed(KeyCode::Right) {
                    adjust_row(rows[self.train_row], &mut self.train, 1);
                } else if clicked || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                    let deg_per_count = games::deg_per_count(self.train_cm, self.dpi);
                    self.test = Some(AimTest::new(deg_per_count, self.mode, self.train));
                    self.grab(true);
                    self.screen = Screen::Playing;
                }
            }
            Screen::TrainingResult => {
                if escape {
                    self.screen = Screen::MainMenu;
                } else if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) || chars.contains(&'r') {
                    self.screen = Screen::TrainingIntro;
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
                match test.feedback {
                    aim::Feedback::Hit => self.sfx.hit(),
                    aim::Feedback::Headshot => self.sfx.head(),
                    aim::Feedback::Miss => self.sfx.miss(),
                    aim::Feedback::None => {}
                }
                if test.phase == Phase::Done {
                    self.sfx.done();
                    let stats = test.stats.clone();
                    let kind = test.kind;
                    self.test = None;
                    self.grab(false);
                    if kind != TestKind::Calibration {
                        self.finish_training(stats);
                        return false;
                    }
                    self.results[self.slot] = Some(stats);
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
                    self.in_routine = false;
                    self.screen = Screen::MainMenu;
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
                    self.screen = Screen::MainMenu;
                }
            }
            Screen::Final => {
                if chars.contains(&'s') {
                    self.save();
                } else if chars.contains(&'r') {
                    self.notice = None;
                    self.screen = Screen::Setup;
                } else if escape {
                    self.notice = None;
                    self.screen = Screen::MainMenu;
                }
            }
            Screen::Help => {
                if escape || clicked || is_key_pressed(KeyCode::F1) || is_key_pressed(KeyCode::Enter) {
                    self.screen = self.help_return;
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
            Screen::MainMenu => self.draw_menu("AIM CALIBRATION", "Sensibilité et entraînement pour vos jeux FPS", MENU, self.menu_sel),
            Screen::TrainMenu => self.draw_menu("ENTRAÎNEMENT", "Choisissez un exercice", TRAIN_MENU, self.train_sel),
            Screen::Setup => self.draw_setup(),
            Screen::Crosshair => self.draw_crosshair_screen(),
            Screen::History => self.draw_history(),
            Screen::RoutineBrief => self.draw_routine_brief(),
            Screen::RoutineSummary => self.draw_routine_summary(),
            Screen::TrainingIntro => self.draw_training_intro(),
            Screen::TrainingResult => self.draw_training_result(),
            Screen::Intro => self.draw_intro(),
            Screen::RoundResult => self.draw_round_result(),
            Screen::Final => self.draw_final(),
            Screen::Help => self.draw_help(),
            Screen::DpiMeasure => self.draw_dpi_measure(),
        }
    }

    fn draw_hud(&self) {
        aim::draw_crosshair(&self.crosshair);
        let Some(test) = &self.test else { return };
        let label = match test.kind {
            TestKind::Calibration => {
                format!("Manche {}/{}  ·  Sensibilité {}", self.round + 1, ROUNDS, slot_name(self.slot))
            }
            kind => format!("{}  ·  {}", mode_name(kind), settings_summary(&self.train)),
        };
        self.ui.text(&label, 30.0, 50.0, 28.0, GREY);
        self.ui.center(&test.status(), 70.0, 34.0, WHITE_SOFT);
        let live = match test.kind {
            TestKind::Flick | TestKind::Micro | TestKind::Switching if test.stats.shots > 0 => {
                format!("Précision {:.0} %", test.stats.accuracy() * 100.0)
            }
            TestKind::Reflex if !test.stats.flick_ms.is_empty() => format!("Réaction {:.0} ms", test.stats.avg_flick_ms()),
            TestKind::Tracking if test.stats.track_total > 0.0 => {
                format!("Sur la cible {:.0} %", test.stats.tracking_pct())
            }
            _ => String::new(),
        };
        let x = screen_width() - self.ui.width(&live, 28.0) - 30.0;
        self.ui.text(&live, x, 50.0, 28.0, GREY);
        self.ui.text("Échap : pause", 30.0, 1050.0, 22.0, GREY);
    }

    fn draw_setup(&self) {
        dim(0.75);
        let ui = &self.ui;
        ui.center(mode_name(self.mode), 200.0, 80.0, ACCENT);
        let subtitle = if self.in_routine {
            "La routine de 10 minutes utilise la sensibilité indiquée ici"
        } else if self.mode == TestKind::Calibration {
            "Indiquez vos réglages actuels : le test partira de là"
        } else {
            "Avec quelle sensibilité voulez-vous vous entraîner ?"
        };
        ui.center(subtitle, 260.0, 32.0, GREY);

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

        let help = match self.mode {
            TestKind::Calibration => [
                format!("{ROUNDS} manches. À chaque manche, deux sensibilités sont testées à l'aveugle (A et B) :"),
                format!("{FLICK_TARGETS} cibles à éliminer en flick, puis {TRACK_DURATION:.0} s de tracking. Choisissez la meilleure, l'écart se resserre."),
            ],
            _ => [
                "L'entraînement utilise exactement la sensibilité indiquée ci-dessus.".to_owned(),
                "Après un calibrage (« Trouver ma sensibilité »), votre sensibilité idéale est pré-remplie.".to_owned(),
            ],
        };
        ui.center(&help[0], 820.0, 26.0, GREY);
        ui.center(&help[1], 856.0, 26.0, GREY);
        ui.center(
            "↑ ↓ : champ  ·  ← → : jeu  ·  Entrée : continuer  ·  H / F1 : aide  ·  M : mesurer le DPI  ·  Échap : menu principal",
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
            ui.center(notice, 920.0, 26.0, ACCENT);
        }
        ui.center(
            "Cette sensibilité est pré-remplie pour les entraînements Flick et Tracking du menu principal.",
            970.0,
            24.0,
            GREY,
        );
        ui.center(
            "S : sauvegarder  ·  R : affiner à partir de ce résultat  ·  Échap : menu principal",
            1020.0,
            28.0,
            GOLD,
        );
    }
}

impl App {
    fn draw_menu(&self, title: &str, subtitle: &str, items: &[(&str, &str)], sel: usize) {
        dim(0.6);
        let ui = &self.ui;
        let s = Ui::scale();
        ui.center(title, 120.0, 84.0, ACCENT);
        ui.center(subtitle, 175.0, 28.0, GREY);

        for (i, (label, desc)) in items.iter().enumerate() {
            let rect = menu_rect(i, items.len());
            let selected = i == sel;
            let (fill, border) = if selected {
                (Color::new(0.0, 1.0, 0.78, 0.16), ACCENT)
            } else {
                (Color::new(0.08, 0.09, 0.13, 0.85), Color::new(1.0, 1.0, 1.0, 0.12))
            };
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, border);
            let cx = rect.x + rect.w / 2.0;
            let top = rect.y / s;
            let title_size = if items.len() > 6 { 30.0 } else { 34.0 };
            ui.center_at(label, cx, top + rect.h / s * 0.42, title_size, if selected { WHITE_SOFT } else { GREY });
            ui.center_at(desc, cx, top + rect.h / s * 0.78, 20.0, GREY);
        }

        ui.center(
            "↑ ↓ ou souris : choisir  ·  Entrée ou clic : valider  ·  F11 : plein écran  ·  Échap : retour",
            1030.0,
            24.0,
            GREY,
        );
    }

    fn draw_crosshair_screen(&self) {
        dim(0.72);
        let ui = &self.ui;
        ui.center("VISEUR", 110.0, 60.0, ACCENT);
        let preview_y = 250.0 * Ui::scale();
        aim::draw_crosshair_at(&self.crosshair, screen_width() / 2.0, preview_y);
        draw_circle_lines(screen_width() / 2.0, preview_y, 64.0 * Ui::scale(), 1.0, GREY);

        let rows = [
            ("Couleur", XHAIR_COLORS[self.crosshair.color].0),
            ("Écart", XHAIR_GAPS[self.crosshair.gap].0),
            ("Longueur", XHAIR_LENGTHS[self.crosshair.length].0),
            ("Épaisseur", XHAIR_THICK[self.crosshair.thick].0),
            ("Point central", if self.crosshair.dot { "Oui" } else { "Non" }),
        ];
        let left = screen_width() / 2.0 - 360.0 * Ui::scale();
        let right = screen_width() / 2.0 + 80.0 * Ui::scale();
        for (i, (label, value)) in rows.iter().enumerate() {
            let y = 560.0 + i as f32 * 58.0;
            let selected = i == self.xhair_row;
            let color = if selected { WHITE_SOFT } else { GREY };
            ui.text(label, left, y, 30.0, color);
            ui.text(&format!("<  {value}  >"), right, y, 30.0, if selected { ACCENT } else { color });
        }
        ui.center("↑ ↓ : réglage  ·  ← → : modifier  ·  Entrée ou Échap : retour", 1020.0, 26.0, GOLD);
    }

    fn draw_history(&self) {
        dim(0.82);
        let ui = &self.ui;
        let s = Ui::scale();
        let names = ["Flick", "Tracking", "Micro-ajustements", "Switching", "Réflexes"];
        ui.center("HISTORIQUE", 120.0, 64.0, ACCENT);
        ui.center(&format!("<  {}  >", names[self.history_mode]), 190.0, 36.0, GOLD);

        let scores: Vec<&Entry> = self.history.iter().filter(|entry| entry.mode == self.history_mode).collect();
        if scores.is_empty() {
            ui.center("Pas encore de session pour cet exercice.", 420.0, 32.0, WHITE_SOFT);
        } else {
            let shown: Vec<&Entry> = scores.iter().rev().take(16).copied().collect::<Vec<_>>().into_iter().rev().collect();
            let max = shown.iter().map(|entry| entry.score).fold(1.0f32, f32::max);
            let left = screen_width() * 0.18;
            let width = screen_width() * 0.64;
            let base = 620.0 * s;
            let height = 250.0 * s;
            draw_line(left, base, left + width, base, 1.0, GREY);
            if shown.len() == 1 {
                let y = base - (shown[0].score / max) * height;
                draw_circle(left + width / 2.0, y, 5.0, ACCENT);
            }
            for pair in shown.windows(2) {
                let x = |i: usize| left + width * i as f32 / (shown.len() - 1) as f32;
                let y = |entry: &Entry| base - (entry.score / max) * height;
                let i = shown.iter().position(|entry| std::ptr::eq(*entry, pair[0])).unwrap_or(0);
                draw_line(x(i), y(pair[0]), x(i + 1), y(pair[1]), 3.0, ACCENT);
                draw_circle(x(i), y(pair[0]), 4.0, WHITE_SOFT);
                draw_circle(x(i + 1), y(pair[1]), 4.0, WHITE_SOFT);
            }
            ui.center(&format!("Meilleur score affiché : {max:.0}"), 680.0, 26.0, GREY);
            for (i, entry) in scores.iter().rev().take(5).enumerate() {
                ui.center(
                    &format!("{}   ·   {:.0}   ·   {}", ago(entry.time), entry.score, entry.note),
                    760.0 + i as f32 * 36.0,
                    24.0,
                    WHITE_SOFT,
                );
            }
        }
        ui.center("← → : exercice  ·  Échap ou clic : retour", 1020.0, 26.0, GOLD);
    }

    fn draw_routine_brief(&self) {
        dim(0.7);
        let ui = &self.ui;
        ui.center(&format!("Routine  ·  étape {} / {}", self.routine_i + 1, ROUTINE.len()), 340.0, 36.0, GREY);
        ui.center(mode_name(self.mode), 450.0, 80.0, ACCENT);
        ui.center("2 minutes  ·  difficulté Normale", 540.0, 32.0, WHITE_SOFT);
        let desc = match self.mode {
            TestKind::Tracking => "Gardez le viseur sur la cible le plus longtemps possible.",
            TestKind::Micro => "De petites cibles apparaissent près du viseur.",
            TestKind::Switching => "Touchez la cible rouge, puis l'autre, et ainsi de suite.",
            TestKind::Reflex => "Attendez l'apparition, puis tirez le plus vite possible.",
            _ => "Éliminez un maximum de cibles, en visant la tête quand elle est visible.",
        };
        ui.center(desc, 620.0, 28.0, GREY);
        ui.center("Cliquez pour commencer", 740.0, 36.0, GOLD);
        ui.center("Échap : abandonner la routine", 1020.0, 24.0, GREY);
    }

    fn draw_routine_summary(&self) {
        dim(0.8);
        let ui = &self.ui;
        ui.center("Routine terminée", 180.0, 64.0, ACCENT);
        let mut total = 0.0;
        for (i, (kind, score)) in self.routine_scores.iter().enumerate() {
            total += score;
            ui.center(&format!("{}    {score:.0}", mode_name(*kind)), 340.0 + i as f32 * 64.0, 36.0, WHITE_SOFT);
        }
        let average = if self.routine_scores.is_empty() { 0.0 } else { total / self.routine_scores.len() as f32 };
        ui.center(&format!("Moyenne  {average:.0}"), 720.0, 48.0, GOLD);
        ui.center("Refaites-la demain : la régularité compte plus qu'une grosse session.", 820.0, 26.0, GREY);
        ui.center("Entrée, clic ou Échap : menu principal", 1020.0, 28.0, GOLD);
    }

    fn draw_training_intro(&self) {
        dim(0.7);
        let ui = &self.ui;
        let s = Ui::scale();
        ui.center(mode_name(self.mode), 150.0, 70.0, ACCENT);
        let desc = match self.mode {
            TestKind::Tracking => "Gardez le viseur sur la cible : elle devient verte quand vous êtes dessus.",
            TestKind::Micro => "Les cibles naissent près du viseur. Travaillez les petites corrections.",
            TestKind::Switching => "Seule la cible rouge vif compte : alternez entre les deux.",
            TestKind::Reflex => "Ne tirez pas trop tôt. La cible apparaît après un délai aléatoire.",
            _ => "Éliminez un maximum de cibles rouges. Chaque tir manqué fait baisser la précision.",
        };
        ui.center(desc, 215.0, 28.0, WHITE_SOFT);

        let left = screen_width() / 2.0 - 440.0 * s;
        let right = screen_width() / 2.0 + 60.0 * s;
        for (i, row) in training_rows(self.mode).iter().enumerate() {
            let y = 280.0 + i as f32 * 52.0;
            let selected = i == self.train_row;
            if selected {
                draw_rectangle(left - 20.0 * s, (y - 42.0) * s, 880.0 * s, 58.0 * s, Color::new(0.0, 1.0, 0.78, 0.08));
            }
            let color = if selected { WHITE_SOFT } else { GREY };
            ui.text(row_label(*row), left, y, 30.0, color);
            let value = format!("<  {}  >", row_value(*row, &self.train));
            ui.text(&value, right, y, 30.0, if selected { ACCENT } else { color });
        }

        let record = match self.record(&self.train) {
            Some(r) if r > 0.0 => format!("Record ({}) : {r:.0}", settings_summary(&self.train)),
            Some(_) => "Pas encore de record pour cette difficulté et cette durée".to_owned(),
            None => "Mode personnalisé : les records ne comptent que pour les difficultés prédéfinies".to_owned(),
        };
        ui.center(&record, 720.0, 26.0, GOLD);
        ui.center(
            &format!("Sensibilité : {:.1} cm/360° à {:.0} DPI", self.train_cm, self.dpi),
            765.0,
            24.0,
            GREY,
        );
        ui.center("Cliquez ou appuyez sur Entrée pour commencer", 860.0, 36.0, GOLD);
        ui.center(
            "↑ ↓ : réglage  ·  ← → : modifier  ·  Échap : retour aux réglages de sensibilité",
            1020.0,
            24.0,
            GREY,
        );
    }

    fn draw_training_result(&self) {
        dim(0.8);
        let ui = &self.ui;
        let Some(stats) = &self.training_result else { return };
        ui.center(
            &format!("{}  ·  {}", mode_name(self.mode), settings_summary(&self.result_settings)),
            200.0,
            44.0,
            GREY,
        );
        ui.center(&format!("{:.0}", stats.training_score(self.mode)), 330.0, 120.0, ACCENT);
        let formula = match self.mode {
            TestKind::Tracking => "score = pourcentage du temps sur la cible × 100",
            TestKind::Reflex => "score = tirs réussis × (400 / temps de réaction) × précision × 100",
            _ => "score = tirs réussis × précision × 100, + 25 par headshot",
        };
        ui.center(formula, 375.0, 22.0, GREY);

        match (self.new_record, self.record(&self.result_settings)) {
            (true, _) => ui.center("Nouveau record !", 440.0, 40.0, GOLD),
            (false, Some(record)) => ui.center(&format!("Record : {record:.0}"), 440.0, 30.0, GREY),
            (false, None) => ui.center("Mode personnalisé : pas de record", 440.0, 26.0, GREY),
        }

        let (lines, tip) = match self.mode {
            TestKind::Tracking => (
                vec![format!("Temps sur la cible : {:.0} %", stats.tracking_pct())],
                if stats.tracking_pct() < 50.0 {
                    "Conseil : gardez un mouvement fluide et réagissez aux changements de direction sans sur-corriger."
                } else {
                    "Conseil : très bon suivi ! Passez à la difficulté supérieure pour progresser."
                },
            ),
            TestKind::Reflex => (
                vec![
                    format!("Cibles touchées : {}", stats.hits),
                    format!("Temps de réaction moyen : {:.0} ms", stats.avg_flick_ms()),
                    format!("Départs trop tôt : {}", stats.false_starts),
                    format!("Cibles manquées : {}", stats.expired),
                ],
                if stats.avg_flick_ms() > 350.0 {
                    "Conseil : restez détendu et commencez le mouvement dès que la cible apparaît, sans la vérifier."
                } else {
                    "Conseil : excellent temps de réaction. Réduisez la taille des cibles pour aller plus loin."
                },
            ),
            _ => (
                vec![
                    format!("Cibles touchées : {}", stats.hits),
                    format!("Dont headshots : {}", stats.headshots),
                    format!("Précision : {:.0} %", stats.accuracy() * 100.0),
                    format!("Temps moyen par cible : {:.0} ms", stats.avg_flick_ms()),
                    format!("Cibles disparues sans être touchées : {}", stats.expired),
                ],
                if stats.accuracy() < 0.8 {
                    "Conseil : visez la précision avant la vitesse, la rapidité viendra avec la régularité."
                } else {
                    "Conseil : bonne précision ! Essayez d'enchaîner plus vite, ou passez à la difficulté supérieure."
                },
            ),
        };
        for (i, line) in lines.iter().enumerate() {
            ui.center(line, 540.0 + i as f32 * 50.0, 32.0, WHITE_SOFT);
        }
        ui.center(tip, 760.0, 26.0, GREY);
        ui.center("Entrée ou R : rejouer  ·  Échap : menu principal", 1020.0, 28.0, GOLD);
    }

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

fn menu_rect(item: usize, count: usize) -> Rect {
    let s = Ui::scale();
    let h = if count > 6 { 78.0 } else { 92.0 };
    let step = h + 14.0;
    let start = 230.0;
    let w = 820.0 * s;
    Rect::new((screen_width() - w) / 2.0, (start + item as f32 * step) * s, w, h * s)
}

fn ago(time: f64) -> String {
    let secs = (miniquad::date::now() - time).max(0.0);
    if secs < 90.0 {
        "à l'instant".to_owned()
    } else if secs < 3600.0 {
        format!("il y a {:.0} min", secs / 60.0)
    } else if secs < 86400.0 {
        format!("il y a {:.0} h", secs / 3600.0)
    } else {
        format!("il y a {:.0} j", secs / 86400.0)
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
    let mut app = App::new(Sfx::load().await);
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

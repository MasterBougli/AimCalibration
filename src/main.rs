#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod aim;
mod games;
mod mouse_info;
mod persist;
mod sfx;
mod win;

use aim::{
    AimTest, Crosshair, DEFAULT_DURATION, DURATIONS, FLICK_TARGETS, LIFETIMES, MOTIONS, MissReplay, NORMAL, PRESETS,
    Phase, SIZES, SPEEDS, Settings, Stats, TRACK_DURATION, TestKind, XHAIR_COLORS, XHAIR_GAPS, XHAIR_LENGTHS,
    XHAIR_THICK, ZONES,
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
    ("Trouver ma sensibilité", "La sensibilité change pendant le flick et le tracking, jusqu'à la bonne"),
    ("Entraînement", "Flick, tracking, micro-ajustements, switching, réflexes et routine"),
    ("Historique", "Scores et sensibilités trouvées au fil du temps"),
    ("Viseur", "Couleur, écart, longueur, épaisseur et point central"),
    ("Options", "Volume des sons et inversion de l'axe vertical"),
    ("Aide", "DPI, cm/360°, eDPI : tout comprendre en une minute"),
    ("Quitter", "À bientôt !"),
];

const TRAIN_MENU: &[(&str, &str)] = &[
    ("Flick", "De grands mouvements pour éliminer des cibles éloignées"),
    ("Tracking", "La cible strafe : elle change de direction d'un coup"),
    ("Micro-ajustements", "De toutes petites corrections autour du viseur"),
    ("Switching", "Alternez rapidement entre deux cibles"),
    ("Réflexes", "Tirez dès que la cible apparaît"),
    ("Peeks", "La cible sort d'un mur : tirez pendant qu'elle est visible"),
    ("Flick en mouvement", "Le viseur dérive, puis une cible apparaît"),
    ("Contre-strafe", "La cible s'arrête net : tirez seulement à ce moment"),
    ("Défi du jour", "Les mêmes cibles chaque jour, pour comparer à hier"),
    ("Échauffement", "S'arrête dès que vous retrouvez votre niveau récent"),
    ("Routine du jour", "10 minutes : flick, vos points faibles, et 15 s de repos entre chaque"),
    ("Retour", "Revenir au menu principal"),
];

const ROUTINE: &[TestKind] = &[TestKind::Flick, TestKind::Tracking, TestKind::Micro, TestKind::Switching, TestKind::Reflex];

/// Records par mode, difficulté et durée. Modes 6 et 7 : flick en mouvement, contre-strafe.
type Records = [[[f32; 4]; 4]; 8];

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
        TestKind::Peek => &[Row::Preset, Row::Duration, Row::Size, Row::Speed, Row::Shape],
        TestKind::Warmup => &[Row::Preset, Row::Size, Row::Shape],
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
        TestKind::Peek => Some(5),
        TestKind::Duel => Some(6),
        TestKind::Counter => Some(7),
        TestKind::Calibration | TestKind::Warmup | TestKind::Live | TestKind::Confirm => None,
    }
}

fn history_index(kind: TestKind) -> Option<usize> {
    match kind {
        TestKind::Warmup => Some(6),
        TestKind::Duel => Some(7),
        TestKind::Counter => Some(8),
        _ => record_index(kind),
    }
}

fn load_records() -> Records {
    let mut records = [[[0.0; 4]; 4]; 8];
    let Ok(text) = std::fs::read_to_string(RECORDS_FILE) else {
        return records;
    };
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let [mode, preset, duration, score] = parts[..]
            && let (Ok(mode), Ok(preset), Ok(duration), Ok(score)) =
                (mode.parse::<usize>(), preset.parse::<usize>(), duration.parse::<usize>(), score.parse())
            && mode < 8
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
        TestKind::Live => "Réglage en direct",
        TestKind::Flick => "Flick",
        TestKind::Tracking => "Tracking",
        TestKind::Micro => "Micro-ajustements",
        TestKind::Switching => "Switching",
        TestKind::Reflex => "Réflexes",
        TestKind::Peek => "Peeks",
        TestKind::Warmup => "Échauffement",
        TestKind::Confirm => "Confirmation",
        TestKind::Duel => "Flick en mouvement",
        TestKind::Counter => "Contre-strafe",
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
    Options,
    History,
    RoutineBrief,
    RoutineRest,
    RoutineSummary,
    Intro,
    Playing,
    Paused,
    RoundResult,
    ConfirmIntro,
    ConfirmChoice,
    Final,
    Transfer,
    Help,
    DpiMeasure,
}

const DAILY_SEED: u64 = 0xA11_DA17;

const TRANSFER_STEPS: [&str; 4] = [
    "J'ai ouvert les options souris du jeu",
    "J'ai collé la sensibilité (touche C sur le résultat)",
    "L'accélération de la souris est désactivée dans le jeu",
    "L'entrée brute est activée, si le jeu la propose",
];

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
    v_ratio: f32,
    ads_ratio: f32,
    invert_y: bool,
    volume: u8,
    options_row: usize,
    worst_miss: Option<MissReplay>,
    week: i64,
    week_done: u32,
    week_goal: u32,
    warmup_rate: f32,
    warmup_reached: bool,
    live_note: Option<String>,
    session_stats: Option<Stats>,
    pointer_accel: bool,
    game_tunes: Vec<persist::GameTune>,
    routine_plan: Vec<TestKind>,
    routine_focus: Vec<bool>,
    rest_until: f64,
    sens_history: Vec<persist::SensEntry>,
    live_path_cm: Vec<f32>,
    confirm_cm: [f32; 2],
    confirm_order: [usize; 2],
    confirm_slot: usize,
    confirm_stats: [Option<Stats>; 2],
    last_elapsed: f32,
    in_routine: bool,
    in_daily: bool,
    transfer: u8,
    transfer_row: usize,
    pointer_speed: u32,
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
    v_sens_text: String,
    ads_text: String,
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
            v_ratio: 1.0,
            ads_ratio: 1.0,
            invert_y: profile.invert_y,
            volume: profile.volume,
            options_row: 0,
            worst_miss: None,
            week: profile.week,
            week_done: profile.week_done,
            week_goal: profile.week_goal,
            warmup_rate: 0.0,
            warmup_reached: false,
            live_note: None,
            session_stats: None,
            pointer_accel: win::enhance_pointer_precision(),
            game_tunes: profile.games,
            routine_plan: Vec::new(),
            routine_focus: Vec::new(),
            rest_until: 0.0,
            sens_history: persist::load_sens_history(),
            live_path_cm: Vec::new(),
            confirm_cm: [0.0, 0.0],
            confirm_order: [0, 1],
            confirm_slot: 0,
            confirm_stats: [None, None],
            last_elapsed: 0.0,
            in_routine: false,
            in_daily: false,
            transfer: profile.transfer,
            transfer_row: 0,
            pointer_speed: win::pointer_speed(),
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
            v_sens_text: profile.vsens,
            ads_text: profile.ads,
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
        let vsens = parse(&self.v_sens_text).unwrap_or(1.0);
        if !(0.1..=2.0).contains(&vsens) {
            self.error = Some("La sensibilité verticale doit être comprise entre 0,1 et 2.".to_owned());
            return None;
        }
        self.v_ratio = vsens;
        let ads = parse(&self.ads_text).unwrap_or(1.0);
        if !(0.1..=2.0).contains(&ads) {
            self.error = Some("La sensibilité en visée doit être comprise entre 0,1 et 2.".to_owned());
            return None;
        }
        self.ads_ratio = ads;
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

    fn start(&mut self, live: bool) {
        let Some(cm) = self.read_settings() else { return };
        match self.mode {
            TestKind::Calibration if live => {
                self.base_cm = cm;
                self.train_cm = cm;
                self.live_note = None;
                self.spawn_test_at(cm, TestKind::Live, self.train);
                self.grab(true);
                self.screen = Screen::Playing;
            }
            TestKind::Calibration => {
                self.base_cm = cm;
                self.live_note = None;
                self.round = 0;
                self.lo = 0.5;
                self.hi = 1.5;
                self.new_round();
            }
            _ => {
                self.train_cm = cm;
                if self.in_daily {
                    self.train = Settings::preset(NORMAL, DEFAULT_DURATION);
                    self.mode = TestKind::Flick;
                    self.screen = Screen::Playing;
                    self.spawn_test(cm);
                    self.grab(true);
                } else if self.in_routine {
                    self.routine_i = 0;
                    self.routine_scores.clear();
                    self.build_routine();
                    self.mode = self.routine_plan[0];
                    self.train = routine_settings();
                    self.screen = Screen::RoutineBrief;
                } else {
                    self.screen = Screen::TrainingIntro;
                }
            }
        }
        self.save_profile();
    }

    fn spawn_test(&mut self, cm: f32) {
        self.spawn_test_at(cm, self.mode, self.train);
    }

    fn spawn_test_at(&mut self, cm: f32, kind: TestKind, settings: Settings) {
        if kind == TestKind::Warmup {
            self.warmup_reached = false;
            let rates: Vec<f32> = self
                .history
                .iter()
                .filter(|entry| entry.mode == 0 && entry.duration > 1.0)
                .rev()
                .take(5)
                .map(|entry| entry.score / entry.duration)
                .collect();
            self.warmup_rate = if rates.is_empty() { 0.0 } else { rates.iter().sum::<f32>() / rates.len() as f32 * 0.9 };
        }
        let deg_per_count = games::deg_per_count(cm, self.dpi);
        if self.in_daily && kind == TestKind::Flick {
            srand(DAILY_SEED);
        }
        self.worst_miss = None;
        self.test = Some(AimTest::new(deg_per_count, kind, settings, self.v_ratio, self.ads_ratio, self.invert_y));
    }

    fn bump_week(&mut self) {
        let week = persist::current_week();
        if self.week != week {
            self.week = week;
            self.week_done = 0;
        }
        self.week_done = self.week_done.saturating_add(1);
    }

    fn cycle_goal(&mut self, delta: i32) {
        let goals = persist::WEEK_GOALS;
        let index = goals.iter().position(|goal| *goal == self.week_goal).unwrap_or(1);
        self.week_goal = goals[cycle(index, goals.len(), delta)];
        self.save_profile();
    }

    fn update_options(&mut self, escape: bool) {
        if escape || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            self.sfx.set_volume(self.volume);
            self.save_profile();
            self.screen = Screen::MainMenu;
            return;
        }
        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::Tab) || is_key_pressed(KeyCode::Up) {
            self.options_row = 1 - self.options_row;
        }
        let delta = if is_key_pressed(KeyCode::Right) { 1 } else if is_key_pressed(KeyCode::Left) { -1 } else { 0 };
        if delta == 0 {
            return;
        }
        if self.options_row == 0 {
            self.invert_y = !self.invert_y;
        } else {
            self.volume = (self.volume as i16 + delta * 10).clamp(0, 100) as u8;
            self.sfx.set_volume(self.volume);
            self.sfx.hit();
        }
        self.save_profile();
    }

    fn view_fov(&self) -> f32 {
        if self.test.as_ref().is_some_and(|test| test.zoomed) { self.fov / aim::ADS_ZOOM } else { self.fov }
    }

    fn remember_game(&mut self) {
        let slot = self.game.min(self.game_tunes.len().saturating_sub(1));
        if let Some(tune) = self.game_tunes.get_mut(slot) {
            tune.sens = self.sens_text.clone();
            tune.fov = self.fov_text.clone();
            tune.vsens = self.v_sens_text.clone();
            tune.ads = self.ads_text.clone();
        }
    }

    fn apply_game(&mut self) {
        let slot = self.game.min(self.game_tunes.len().saturating_sub(1));
        if let Some(tune) = self.game_tunes.get(slot) {
            self.sens_text = tune.sens.clone();
            self.fov_text = tune.fov.clone();
            self.v_sens_text = tune.vsens.clone();
            self.ads_text = tune.ads.clone();
        }
    }

    /// Flick, puis les deux exercices les plus loin de leur record, chacun retravaillé.
    fn build_routine(&mut self) {
        const POOL: [TestKind; 5] = [
            TestKind::Tracking,
            TestKind::Micro,
            TestKind::Switching,
            TestKind::Reflex,
            TestKind::Peek,
        ];
        let mut ranked = Vec::new();
        for kind in POOL {
            let Some(mode) = record_index(kind) else { continue };
            let scores: Vec<f32> = self.history.iter().filter(|entry| entry.mode == mode).map(|entry| entry.score).collect();
            if scores.len() < 2 {
                continue;
            }
            let best = scores.iter().copied().fold(0.0f32, f32::max);
            if best <= 0.0 {
                continue;
            }
            let n = scores.len().min(5);
            let recent = scores[scores.len() - n..].iter().sum::<f32>() / n as f32;
            ranked.push((kind, recent / best));
        }
        ranked.sort_by(|a, b| a.1.total_cmp(&b.1));
        let plan = if ranked.len() >= 2 {
            vec![TestKind::Flick, ranked[0].0, ranked[1].0, ranked[0].0, TestKind::Flick]
        } else if let Some((kind, _)) = ranked.first() {
            let filler = POOL.into_iter().find(|item| item != kind).unwrap_or(TestKind::Tracking);
            vec![TestKind::Flick, *kind, filler, *kind, TestKind::Flick]
        } else {
            ROUTINE.to_vec()
        };
        let weak: Vec<TestKind> = ranked.iter().take(2).map(|(kind, _)| *kind).collect();
        self.routine_focus = if ranked.is_empty() {
            vec![false; plan.len()]
        } else {
            plan.iter().enumerate().map(|(i, kind)| i != 0 && i + 1 != plan.len() && weak.contains(kind)).collect()
        };
        self.routine_plan = plan;
    }

    fn save_profile(&mut self) {
        self.remember_game();
        persist::save_profile(&persist::Profile {
            dpi: self.dpi_text.clone(),
            sens: self.sens_text.clone(),
            fov: self.fov_text.clone(),
            vsens: self.v_sens_text.clone(),
            ads: self.ads_text.clone(),
            invert_y: self.invert_y,
            volume: self.volume,
            week: self.week,
            week_done: self.week_done,
            week_goal: self.week_goal,
            transfer: self.transfer,
            game: self.game,
            games: self.game_tunes.clone(),
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
            4 => self.screen = Screen::Options,
            5 => self.open_help(),
            _ => return true,
        }
        false
    }

    fn activate_train(&mut self, item: usize) {
        const MODES: [TestKind; 9] = [
            TestKind::Flick,
            TestKind::Tracking,
            TestKind::Micro,
            TestKind::Switching,
            TestKind::Reflex,
            TestKind::Peek,
            TestKind::Duel,
            TestKind::Counter,
            TestKind::Warmup,
        ];
        self.in_daily = false;
        if let Some(mode) = MODES.get(item) {
            self.mode = *mode;
            self.in_routine = false;
            self.error = None;
            self.screen = Screen::Setup;
        } else if item == MODES.len() {
            self.in_daily = true;
            self.in_routine = false;
            self.mode = TestKind::Flick;
            self.error = None;
            self.screen = Screen::Setup;
        } else if item == MODES.len() + 1 {
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
        if !self.in_daily
            && let Some(best) = self.record_slot(&settings)
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
        self.bump_week();
        self.save_profile();
        let history_mode = if self.in_daily { Some(9) } else { history_index(self.mode) };
        if let Some(mode) = history_mode {
            let note = if self.in_daily {
                format!("défi · précision {:.0} %", stats.accuracy() * 100.0)
            } else {
                match self.mode {
                    TestKind::Tracking => format!("suivi {:.0} %", stats.tracking_pct()),
                    TestKind::Reflex => format!("réaction {:.0} ms", stats.avg_flick_ms()),
                    _ => format!("précision {:.0} %", stats.accuracy() * 100.0),
                }
            };
            let duration = if self.mode == TestKind::Warmup {
                self.last_elapsed.max(1.0)
            } else {
                DURATIONS[self.result_settings.duration].1
            };
            let entry = Entry {
                time: miniquad::date::now(),
                mode,
                score: stats.training_score(self.mode),
                duration,
                note,
            };
            persist::append_history(&entry);
            self.history.push(entry);
        }
        if self.in_routine {
            self.routine_scores.push((self.mode, stats.training_score(self.mode)));
            self.routine_i += 1;
            if self.routine_i >= self.routine_plan.len() {
                self.in_routine = false;
                self.screen = Screen::RoutineSummary;
            } else {
                self.mode = self.routine_plan[self.routine_i];
                self.train = routine_settings();
                self.rest_until = miniquad::date::now() + 15.0;
                self.screen = Screen::RoutineRest;
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
            self.session_stats = None;
            self.live_note = None;
            self.live_path_cm.clear();
            self.save_profile();
            self.record_sens(self.cm(self.final_mult()));
            self.screen = Screen::Final;
        } else {
            self.new_round();
        }
    }

    fn finish_live(&mut self, stats: Stats, mult: f32, step: f32, locked: bool, path: Vec<f32>) {
        let start = self.base_cm;
        self.live_path_cm = path.into_iter().map(|factor| start / factor.max(0.05)).collect();
        let found = start / mult;
        if self.live_path_cm.last().is_none_or(|cm| (cm - found).abs() > 0.05) {
            self.live_path_cm.push(found);
        }
        if self.live_path_cm.len() == 1 {
            self.live_path_cm.push(found);
        }
        self.base_cm = found;
        self.lo = 1.0;
        self.hi = 1.0;
        self.live_note = Some(if locked {
            format!("Réglage stabilisé pendant le jeu, à environ ±{:.0} %.", (step * 100.0).max(1.0))
        } else {
            "La session s'est terminée avant stabilisation. Relancez pour affiner.".to_owned()
        });
        if self.unknown_game() {
            self.game = 0;
        }
        let sens = games::sens_for(GAMES[self.game].yaw, found, self.dpi);
        self.sens_text = format!("{sens:.3}");
        self.session_stats = Some(stats);
        self.begin_confirm(found);
    }

    fn begin_confirm(&mut self, found: f32) {
        let factor = if gen_range(0.0f32, 1.0) < 0.5 { 1.12 } else { 1.0 / 1.12 };
        self.confirm_cm = [found, found * factor];
        self.confirm_order = if gen_range(0.0f32, 1.0) < 0.5 { [0, 1] } else { [1, 0] };
        self.confirm_slot = 0;
        self.confirm_stats = [None, None];
        self.screen = Screen::ConfirmIntro;
    }

    fn record_sens(&mut self, cm: f32) {
        let entry = persist::SensEntry { time: miniquad::date::now(), cm };
        persist::append_sens(&entry);
        self.sens_history.push(entry);
    }

    fn settle_sens(&mut self, cm: f32) {
        self.base_cm = cm;
        self.lo = 1.0;
        self.hi = 1.0;
        if self.unknown_game() {
            self.game = 0;
        }
        let sens = games::sens_for(GAMES[self.game].yaw, cm, self.dpi);
        self.sens_text = format!("{sens:.3}");
        self.save_profile();
        self.record_sens(cm);
        self.screen = Screen::Final;
    }

    fn skip_confirm(&mut self) {
        if let Some(note) = &mut self.live_note {
            note.push_str(" Confirmation passée.");
        }
        self.settle_sens(self.confirm_cm[0]);
    }

    fn accept_confirm(&mut self, shown: usize) {
        let idx = self.confirm_order[shown];
        let chosen = self.confirm_cm[idx];
        if idx == 0 {
            if let Some(note) = &mut self.live_note {
                note.push_str(" Confirmée à l'aveugle.");
            }
        } else {
            let faster = chosen < self.confirm_cm[0];
            self.live_note = Some(format!(
                "Vous avez préféré la variante, 12 % plus {}. Elle remplace le résultat.",
                if faster { "rapide" } else { "lente" }
            ));
        }
        self.settle_sens(chosen);
    }

    fn copy_result(&mut self) {
        let cm = self.cm(self.final_mult());
        let game = GAMES.get(self.game).unwrap_or(&GAMES[0]);
        let sens = games::sens_for(game.yaw, cm, self.dpi);
        let line = format!("{} · {:.0} DPI · {sens:.3}", game.name, self.dpi);
        self.notice = Some(if win::copy_text(&line) {
            format!("Copié : {line}")
        } else {
            "Impossible de copier dans le presse-papiers.".to_owned()
        });
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
                if is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::KpSubtract) {
                    self.cycle_goal(-1);
                }
                if is_key_pressed(KeyCode::Equal) || is_key_pressed(KeyCode::KpAdd) {
                    self.cycle_goal(1);
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
            Screen::Options => self.update_options(escape),
            Screen::History => {
                if escape || clicked {
                    self.screen = Screen::MainMenu;
                } else if is_key_pressed(KeyCode::Left) {
                    self.history_mode = cycle(self.history_mode, 11, -1);
                } else if is_key_pressed(KeyCode::Right) {
                    self.history_mode = cycle(self.history_mode, 11, 1);
                }
            }
            Screen::RoutineRest => {
                if escape {
                    self.in_routine = false;
                    self.screen = Screen::MainMenu;
                } else if clicked || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) || miniquad::date::now() >= self.rest_until {
                    self.screen = Screen::RoutineBrief;
                }
            }
            Screen::RoutineBrief => {
                if escape {
                    self.in_routine = false;
                    self.screen = Screen::MainMenu;
                } else if clicked || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                    self.spawn_test(self.train_cm);
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
                    self.in_daily = false;
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
                    self.field = (self.field + 1) % 6;
                }
                if is_key_pressed(KeyCode::Up) {
                    self.field = (self.field + 5) % 6;
                }
                if self.field == 1 {
                    let choices = GAMES.len() + 1;
                    if is_key_pressed(KeyCode::Left) {
                        self.remember_game();
                        self.game = (self.game + choices - 1) % choices;
                        self.apply_game();
                    }
                    if is_key_pressed(KeyCode::Right) {
                        self.remember_game();
                        self.game = (self.game + 1) % choices;
                        self.apply_game();
                    }
                }
                let field = self.field;
                let unknown = self.unknown_game();
                let text = match field {
                    0 => Some(&mut self.dpi_text),
                    2 if !unknown => Some(&mut self.sens_text),
                    3 => Some(&mut self.fov_text),
                    4 => Some(&mut self.v_sens_text),
                    5 => Some(&mut self.ads_text),
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
                    self.start(self.mode == TestKind::Calibration);
                } else if chars.contains(&'c') && self.mode == TestKind::Calibration {
                    self.start(false);
                }
            }
            Screen::Intro => {
                if escape {
                    self.screen = Screen::Setup;
                } else if clicked {
                    let cm = self.cm(self.order[self.slot]);
                    self.spawn_test_at(cm, TestKind::Calibration, self.train);
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
                    self.spawn_test(self.train_cm);
                    self.grab(true);
                    self.screen = Screen::Playing;
                }
            }
            Screen::TrainingResult => {
                if escape {
                    self.in_daily = false;
                    self.screen = Screen::MainMenu;
                } else if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) || chars.contains(&'r') {
                    if self.in_daily {
                        self.train = Settings::preset(NORMAL, DEFAULT_DURATION);
                        self.mode = TestKind::Flick;
                        self.spawn_test(self.train_cm);
                        self.grab(true);
                        self.screen = Screen::Playing;
                    } else {
                        self.screen = Screen::TrainingIntro;
                    }
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
                test.update(counts, clicked, is_mouse_button_down(MouseButton::Right), dt);
                match test.feedback {
                    aim::Feedback::Hit => self.sfx.hit(),
                    aim::Feedback::Headshot => self.sfx.head(),
                    aim::Feedback::Miss => self.sfx.miss(),
                    aim::Feedback::None => {}
                }
                if self.mode == TestKind::Warmup && test.phase != Phase::Done {
                    let elapsed = test.elapsed();
                    let rate = if elapsed > 0.0 { test.stats.click_score() / elapsed } else { 0.0 };
                    let ready = if self.warmup_rate > 0.0 { elapsed >= 20.0 && rate >= self.warmup_rate } else { elapsed >= 45.0 };
                    if ready {
                        self.warmup_reached = self.warmup_rate > 0.0;
                        test.request_finish();
                    }
                }
                if test.phase == Phase::Done {
                    self.last_elapsed = test.elapsed();
                    self.sfx.done();
                    let stats = test.stats.clone();
                    let kind = test.kind;
                    let live = (test.live_mult, test.live_step, test.live_locked);
                    let path = test.live_path.clone();
                    self.worst_miss = test.worst.clone();
                    self.test = None;
                    self.grab(false);
                    if kind == TestKind::Live {
                        self.finish_live(stats, live.0, live.1, live.2, path);
                        return false;
                    }
                    if kind == TestKind::Confirm {
                        self.confirm_stats[self.confirm_slot] = Some(stats);
                        if self.confirm_slot == 0 {
                            self.confirm_slot = 1;
                            self.screen = Screen::ConfirmIntro;
                        } else {
                            self.screen = Screen::ConfirmChoice;
                        }
                        return false;
                    }
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
            Screen::ConfirmIntro => {
                if escape {
                    self.skip_confirm();
                } else if clicked || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                    let cm = self.confirm_cm[self.confirm_order[self.confirm_slot]];
                    self.spawn_test_at(cm, TestKind::Confirm, self.train);
                    self.grab(true);
                    self.screen = Screen::Playing;
                }
            }
            Screen::ConfirmChoice => {
                if chars.contains(&'a') || is_key_pressed(KeyCode::Left) {
                    self.accept_confirm(0);
                } else if chars.contains(&'b') || is_key_pressed(KeyCode::Right) {
                    self.accept_confirm(1);
                } else if escape {
                    self.skip_confirm();
                }
            }
            Screen::Final => {
                if chars.contains(&'s') {
                    self.save();
                } else if chars.contains(&'c') {
                    self.copy_result();
                } else if chars.contains(&'t') {
                    self.transfer_row = 0;
                    self.screen = Screen::Transfer;
                } else if chars.contains(&'r') {
                    self.notice = None;
                    self.screen = Screen::Setup;
                } else if escape {
                    self.notice = None;
                    self.screen = Screen::MainMenu;
                }
            }
            Screen::Transfer => self.update_transfer(&chars, escape),
            Screen::Help => {
                if escape || clicked || is_key_pressed(KeyCode::F1) || is_key_pressed(KeyCode::Enter) {
                    self.screen = self.help_return;
                }
            }
            Screen::DpiMeasure => self.update_dpi_measure(&chars, escape),
        }
        false
    }

    fn update_transfer(&mut self, chars: &[char], escape: bool) {
        if escape || is_key_pressed(KeyCode::Enter) && chars.is_empty() {
            self.save_profile();
            self.screen = Screen::Final;
            return;
        }
        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::Tab) {
            self.transfer_row = (self.transfer_row + 1) % TRANSFER_STEPS.len();
        }
        if is_key_pressed(KeyCode::Up) {
            self.transfer_row = (self.transfer_row + TRANSFER_STEPS.len() - 1) % TRANSFER_STEPS.len();
        }
        let digit = chars.iter().find_map(|c| c.to_digit(10)).map(|n| n as usize);
        let toggle = chars.contains(&' ') || is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::Left);
        if let Some(n) = digit.filter(|n| (1..=TRANSFER_STEPS.len()).contains(n)) {
            self.transfer ^= 1 << (n - 1);
            self.transfer_row = n - 1;
            self.save_profile();
        } else if toggle {
            self.transfer ^= 1 << self.transfer_row;
            self.save_profile();
        }
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
        let fov = self.view_fov();
        match (&self.test, self.screen) {
            (Some(test), Screen::Playing | Screen::Paused) => {
                set_camera(&aim::camera(test.yaw, test.pitch, fov));
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
            Screen::MainMenu => {
                self.draw_menu("AIM CALIBRATION", "Sensibilité et entraînement pour vos jeux FPS", MENU, self.menu_sel);
                if self.pointer_accel {
                    self.ui.center(
                        "Accélération Windows activée : décochez « Améliorer la précision du pointeur ».",
                        910.0,
                        22.0,
                        RED,
                    );
                }
                self.draw_week_bar();
            }
            Screen::TrainMenu => self.draw_menu("ENTRAÎNEMENT", "Choisissez un exercice", TRAIN_MENU, self.train_sel),
            Screen::Setup => self.draw_setup(),
            Screen::Crosshair => self.draw_crosshair_screen(),
            Screen::Options => self.draw_options(),
            Screen::History => self.draw_history(),
            Screen::RoutineBrief => self.draw_routine_brief(),
            Screen::RoutineRest => self.draw_routine_rest(),
            Screen::RoutineSummary => self.draw_routine_summary(),
            Screen::TrainingIntro => self.draw_training_intro(),
            Screen::TrainingResult => self.draw_training_result(),
            Screen::Intro => self.draw_intro(),
            Screen::RoundResult => self.draw_round_result(),
            Screen::ConfirmIntro => self.draw_confirm_intro(),
            Screen::ConfirmChoice => self.draw_confirm_choice(),
            Screen::Final => self.draw_final(),
            Screen::Transfer => self.draw_transfer(),
            Screen::Help => self.draw_help(),
            Screen::DpiMeasure => self.draw_dpi_measure(),
        }
        if matches!(self.screen, Screen::Playing | Screen::Paused)
            && let Some(test) = &self.test
        {
            test.draw_miss_path(fov);
        }
    }

    fn draw_hud(&self) {
        aim::draw_crosshair(&self.crosshair);
        let Some(test) = &self.test else { return };
        let label = match test.kind {
            TestKind::Calibration => {
                format!("Manche {}/{}  ·  Sensibilité {}", self.round + 1, ROUNDS, slot_name(self.slot))
            }
            TestKind::Live => "Réglage en direct  ·  visez normalement, sans regarder les chiffres".to_owned(),
            TestKind::Confirm => format!("Confirmation  ·  Sensibilité {}", slot_name(self.confirm_slot)),
            kind => format!("{}  ·  {}", mode_name(kind), settings_summary(&self.train)),
        };
        self.ui.text(&label, 30.0, 50.0, 28.0, GREY);
        self.ui.center(&test.status(), 70.0, 34.0, WHITE_SOFT);
        let live = match test.kind {
            TestKind::Flick | TestKind::Micro | TestKind::Switching if test.stats.shots > 0 => {
                format!("Précision {:.0} %", test.stats.accuracy() * 100.0)
            }
            TestKind::Reflex if !test.stats.flick_ms.is_empty() => format!("Réaction {:.0} ms", test.stats.avg_flick_ms()),
            TestKind::Warmup => {
                let elapsed = test.elapsed().max(0.1);
                let rate = test.stats.click_score() / elapsed;
                if self.warmup_rate > 0.0 {
                    format!("Niveau {:.0} %", (rate / self.warmup_rate * 100.0).min(150.0))
                } else {
                    format!("Référence dans {:.0} s", (45.0 - elapsed).max(0.0))
                }
            }
            TestKind::Tracking if test.stats.track_total > 0.0 => {
                format!("Sur la cible {:.0} %", test.stats.tracking_pct())
            }
            _ => String::new(),
        };
        let x = screen_width() - self.ui.width(&live, 28.0) - 30.0;
        self.ui.text(&live, x, 50.0, 28.0, GREY);
        let zoom = if test.zoomed { "Visée  ·  " } else { "" };
        self.ui.text(&format!("{zoom}Clic droit : zoomer  ·  Échap : pause"), 30.0, 1050.0, 22.0, GREY);
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
            ("Sensibilité verticale (×)", self.v_sens_text.clone()),
            ("Sensibilité en visée (×)", self.ads_text.clone()),
        ];
        let left = screen_width() / 2.0 - 520.0 * Ui::scale();
        let right = screen_width() / 2.0 + 40.0 * Ui::scale();
        for (i, (label, value)) in fields.iter().enumerate() {
            let y = 340.0 + i as f32 * 58.0;
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
            let editable = i == 0 || i >= 3 || (i == 2 && game.is_some());
            let value = if selected && editable { format!("{value}{cursor}") } else { value.clone() };
            ui.text(&value, right, y, 32.0, if selected { ACCENT } else { color });
        }

        let hint = match (self.field, game) {
            (0, _) => "La précision de votre souris (souvent 400, 800 ou 1600). Vous ne la connaissez pas ? Appuyez sur M pour la détecter et la mesurer.".to_owned(),
            (1, _) => "Le jeu dont vous connaissez la sensibilité. Aucune idée ? Choisissez « Je ne sais pas » (← →).".to_owned(),
            (2, Some(g)) => format!("Dans {} : {}", g.name, g.settings),
            (2, None) => "Pas besoin : le test partira d'une valeur moyenne, puis s'adaptera à vous.".to_owned(),
            (3, _) => "L'angle de vue du jeu. Dans le doute, laissez 103 (valeur fixe de Valorant).".to_owned(),
            (4, _) => "Multiplicateur vertical, comme dans Valorant. 1 = même vitesse que l'horizontale.".to_owned(),
            _ => "Multiplicateur quand vous visez. En jeu, maintenez le clic droit : le zoom est de ×1,5.".to_owned(),
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
        } else if self.pointer_accel {
            ui.center(
                "Accélération Windows activée : décochez « Améliorer la précision du pointeur », sinon le résultat sera faux.",
                765.0,
                24.0,
                RED,
            );
        }

        let help = match self.mode {
            TestKind::Calibration => [
                "Entrée : 40 s de flick puis 24 s de tracking. La sensibilité change toute seule selon vos dépassements et votre suivi.".to_owned(),
                format!("C : l'autre méthode, {ROUNDS} manches où vous choisissez entre deux sensibilités."),
            ],
            _ => [
                "L'entraînement utilise exactement la sensibilité indiquée ci-dessus.".to_owned(),
                "Après un calibrage (« Trouver ma sensibilité »), votre sensibilité idéale est pré-remplie.".to_owned(),
            ],
        };
        ui.center(&help[0], 820.0, 26.0, GREY);
        ui.center(&help[1], 856.0, 26.0, GREY);
        let footer = if self.mode == TestKind::Calibration {
            "↑ ↓ : champ  ·  ← → : jeu  ·  Entrée : réglage en direct  ·  C : comparaison en manches  ·  H : aide  ·  M : DPI  ·  Échap : menu"
        } else {
            "↑ ↓ : champ  ·  ← → : jeu  ·  Entrée : continuer  ·  H / F1 : aide  ·  M : mesurer le DPI  ·  Échap : menu principal"
        };
        ui.center(footer, 1020.0, 24.0, GREY);
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

    fn draw_confirm_intro(&self) {
        dim(0.7);
        let ui = &self.ui;
        ui.center("Confirmation à l'aveugle", 300.0, 48.0, GREY);
        ui.center(&format!("Sensibilité {}", slot_name(self.confirm_slot)), 430.0, 90.0, ACCENT);
        ui.center("10 cibles. Vous ne voyez pas le chiffre.", 540.0, 32.0, WHITE_SOFT);
        ui.center("Ensuite vous direz laquelle des deux était la plus naturelle.", 590.0, 28.0, GREY);
        ui.center("Cliquez pour commencer", 720.0, 36.0, GOLD);
        ui.center("Échap : garder le réglage trouvé, sans confirmer", 1020.0, 24.0, GREY);
    }

    fn draw_confirm_choice(&self) {
        dim(0.75);
        let ui = &self.ui;
        ui.center("Laquelle était la plus naturelle ?", 180.0, 48.0, WHITE_SOFT);
        for slot in 0..2 {
            let stats = self.confirm_stats[slot].as_ref();
            let cx = screen_width() / 2.0 + (slot as f32 * 2.0 - 1.0) * 360.0 * Ui::scale();
            ui.center_at(&format!("Sensibilité {}", slot_name(slot)), cx, 340.0, 56.0, ACCENT);
            let accuracy = stats.map(|item| item.accuracy() * 100.0).unwrap_or(0.0);
            let time = stats.map(|item| item.avg_flick_ms()).unwrap_or(0.0);
            ui.center_at(&format!("Précision : {accuracy:.0} %"), cx, 430.0, 28.0, GREY);
            ui.center_at(&format!("Temps moyen : {time:.0} ms"), cx, 476.0, 28.0, GREY);
        }
        ui.center("Le score ne dit pas laquelle est la bonne. Fiez-vous au ressenti.", 700.0, 26.0, GREY);
        ui.center("A ou ← : garder A  ·  B ou → : garder B  ·  Échap : garder le réglage trouvé", 800.0, 28.0, GOLD);
    }

    fn draw_routine_rest(&self) {
        dim(0.72);
        let ui = &self.ui;
        let left = (self.rest_until - miniquad::date::now()).max(0.0).ceil();
        ui.center("Repos", 380.0, 80.0, ACCENT);
        ui.center(&format!("{left:.0} s"), 500.0, 100.0, WHITE_SOFT);
        ui.center("La suite arrive. La fatigue du dernier exercice ne doit pas fausser le suivant.", 640.0, 28.0, GREY);
        ui.center(mode_name(self.mode), 720.0, 36.0, GOLD);
        ui.center("Clic ou Entrée : passer  ·  Échap : abandonner la routine", 1020.0, 24.0, GREY);
    }

    fn draw_final(&self) {
        dim(0.8);
        let ui = &self.ui;
        let mult = self.final_mult();
        let cm = self.cm(mult);
        ui.center("Votre sensibilité idéale", 150.0, 44.0, GREY);
        ui.center(&format!("{cm:.1} cm / 360°"), 250.0, 100.0, ACCENT);
        ui.center(
            &format!(
                "{:.1} pouces / 360°  ·  {:.0} DPI  ·  vertical ×{:.2}  ·  visée ×{:.2}",
                cm / 2.54,
                self.dpi,
                self.v_ratio,
                self.ads_ratio
            ),
            310.0,
            30.0,
            WHITE_SOFT,
        );
        if let Some(note) = &self.live_note {
            ui.center(note, 352.0, 24.0, GOLD);
        }
        self.draw_live_path();
        let mut games_top = 400.0;
        if self.sens_history.len() >= 2 {
            let prev = &self.sens_history[self.sens_history.len() - 2];
            ui.center(
                &format!("Précédente : {:.1} cm/360° · {}", prev.cm, ago(prev.time)),
                games_top,
                22.0,
                GREY,
            );
            games_top += 28.0;
        }
        if let Some(stats) = &self.session_stats {
            if let Some(line) = stats.direction_note() {
                ui.center(&line, games_top, 24.0, WHITE_SOFT);
                games_top += 30.0;
            }
            if let Some(line) = stats.distance_note() {
                ui.center(&line, games_top, 24.0, WHITE_SOFT);
                games_top += 30.0;
            }
            games_top += 16.0;
        }

        let left = screen_width() / 2.0 - 400.0 * Ui::scale();
        let right = screen_width() / 2.0 + 180.0 * Ui::scale();
        for (i, game) in GAMES.iter().enumerate() {
            let y = games_top + i as f32 * 46.0;
            let sens = games::sens_for(game.yaw, cm, self.dpi);
            let color = if i == self.game { GOLD } else { WHITE_SOFT };
            ui.text(game.name, left, y, 32.0, color);
            ui.text(&format!("{sens:.3}"), right, y, 32.0, color);
            ui.text(&format!("eDPI {:.0}", sens * self.dpi), right + 140.0 * Ui::scale(), y, 24.0, GREY);
        }

        let mut y = games_top + GAMES.len() as f32 * 46.0 + 24.0;
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
            &format!(
                "League of Legends n'est pas un FPS. Vitesse du pointeur Windows : {}/20. 10 est le milieu.",
                self.pointer_speed
            ),
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
            "S : sauvegarder  ·  C : copier  ·  T : transfert dans le jeu  ·  R : affiner  ·  Échap : menu",
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

    fn draw_week_bar(&self) {
        let scale = Ui::scale();
        let width = 460.0 * scale;
        let height = 14.0 * scale;
        let x = (screen_width() - width) / 2.0;
        let y = 972.0 * scale;
        let ratio = (self.week_done as f32 / self.week_goal.max(1) as f32).min(1.0);
        draw_rectangle(x, y, width, height, Color::new(1.0, 1.0, 1.0, 0.08));
        draw_rectangle(x, y, width * ratio, height, ACCENT);
        self.ui.center(
            &format!("Objectif de la semaine  {}/{}    ·    - / + pour le modifier", self.week_done, self.week_goal),
            960.0,
            22.0,
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

    fn draw_live_path(&self) {
        let scale = Ui::scale();
        if self.live_path_cm.len() >= 2 {
            let left = 56.0 * scale;
            let base = 360.0 * scale;
            self.ui.text("Pendant le test", 56.0, 220.0, 22.0, GREY);
            draw_samples(&self.live_path_cm, left, base, 240.0 * scale, 90.0 * scale);
            let first = self.live_path_cm[0];
            let last = *self.live_path_cm.last().unwrap();
            self.ui.text(&format!("{first:.0} → {last:.0} cm"), 56.0, 390.0, 20.0, GREY);
        }
        if let Some(stats) = &self.session_stats {
            if stats.impacts.len() >= 3 {
                let cx = 170.0 * scale;
                let cy = 680.0 * scale;
                aim::AimTest::draw_cloud(&stats.impacts, cx, cy, 70.0 * scale);
                self.ui.center_at("Tirs", cx, 560.0, 22.0, GOLD);
            }
        }
    }

    fn draw_sens_history(&self) {
        let ui = &self.ui;
        let scale = Ui::scale();
        if self.sens_history.is_empty() {
            ui.center("Pas encore de calibrage enregistré.", 420.0, 32.0, WHITE_SOFT);
            return;
        }
        let shown: Vec<f32> = self.sens_history.iter().rev().take(16).map(|entry| entry.cm).collect::<Vec<_>>().into_iter().rev().collect();
        if shown.len() == 1 {
            ui.center(&format!("{:.1} cm / 360°", shown[0]), 460.0, 64.0, ACCENT);
        } else {
            let left = screen_width() * 0.18;
            let width = screen_width() * 0.64;
            let base = 620.0 * scale;
            draw_line(left, base, left + width, base, 1.0, GREY);
            draw_samples(&shown, left, base, width, 250.0 * scale);
            let min = shown.iter().copied().fold(f32::MAX, f32::min);
            let max = shown.iter().copied().fold(f32::MIN, f32::max);
            ui.center(&format!("De {min:.1} à {max:.1} cm/360°"), 680.0, 26.0, GREY);
        }
        for (i, entry) in self.sens_history.iter().rev().take(5).enumerate() {
            ui.center(
                &format!("{}   ·   {:.1} cm/360°", ago(entry.time), entry.cm),
                760.0 + i as f32 * 36.0,
                24.0,
                WHITE_SOFT,
            );
        }
    }

    fn draw_history(&self) {
        dim(0.82);
        let ui = &self.ui;
        let s = Ui::scale();
        let names = [
            "Flick",
            "Tracking",
            "Micro-ajustements",
            "Switching",
            "Réflexes",
            "Peeks",
            "Échauffement",
            "Flick en mouvement",
            "Contre-strafe",
            "Défi du jour",
            "Sensibilité",
        ];
        ui.center("HISTORIQUE", 120.0, 64.0, ACCENT);
        ui.center(&format!("<  {}  >", names[self.history_mode.min(names.len() - 1)]), 190.0, 36.0, GOLD);

        if self.history_mode == names.len() - 1 {
            self.draw_sens_history();
            ui.center("← → : exercice  ·  Échap ou clic : retour", 1020.0, 26.0, GOLD);
            return;
        }

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
        ui.center(&format!("Routine  ·  étape {} / {}", self.routine_i + 1, self.routine_plan.len().max(1)), 340.0, 36.0, GREY);
        ui.center(mode_name(self.mode), 450.0, 80.0, ACCENT);
        ui.center("2 minutes  ·  difficulté Normale", 540.0, 32.0, WHITE_SOFT);
        if self.routine_focus.get(self.routine_i).copied().unwrap_or(false) {
            ui.center("Point faible : vous êtes loin de votre meilleur score sur cet exercice.", 590.0, 26.0, GOLD);
        }
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
            TestKind::Peek => "La cible dépasse du mur quelques instants. Tirez pendant qu'elle est à découvert.",
            TestKind::Duel => "Le viseur dérive tout seul, puis une cible apparaît devant le mouvement.",
            TestKind::Counter => "Tirez seulement quand la cible devient verte : elle est alors immobile.",
            TestKind::Warmup => "L'exercice s'arrête dès que votre rythme retrouve vos dernières sessions de flick.",
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
        let mut lines = lines;
        if let Some(spread) = stats.spread_ms() {
            let label = if spread < 80.0 {
                "très régulier"
            } else if spread < 180.0 {
                "correct"
            } else {
                "irrégulier"
            };
            lines.push(format!("Régularité : {label}  ·  {spread:.0} ms entre les flicks rapides et les lents"));
        }
        if stats.traces > 0 {
            let tremor = if stats.avg_tremor() < 0.15 {
                "faible"
            } else if stats.avg_tremor() < 0.45 {
                "présent"
            } else {
                "fort"
            };
            lines.push(format!("Dépassement de la cible : {:.1}°", stats.avg_overshoot()));
            lines.push(format!("Tremblement en fin de geste : {tremor}"));
            lines.push(format!("Temps d'ajustement avant le clic : {:.0} ms", stats.avg_correction()));
        }
        if let Some(line) = stats.direction_note() {
            lines.push(line);
        }
        if let Some(line) = stats.distance_note() {
            lines.push(line);
        }
        if self.mode == TestKind::Warmup {
            let message = if self.warmup_reached {
                "Niveau récent retrouvé : vous pouvez lancer une partie."
            } else if self.warmup_rate == 0.0 {
                "Pas encore de sessions de flick : celle-ci servira de référence."
            } else {
                "Temps maximum atteint, le niveau récent n'est pas encore là."
            };
            lines.push(message.to_owned());
        }
        if self.in_daily {
            let (yesterday, best) = self.daily_scores();
            lines.push(match yesterday {
                Some(score) => format!("Hier, sur les mêmes cibles : {score:.0}"),
                None => "Pas encore de défi hier sur ce scénario.".to_owned(),
            });
            if let Some(score) = best {
                lines.push(format!("Record du scénario : {score:.0}"));
            }
        }
        for (i, line) in lines.iter().enumerate() {
            ui.center(line, 500.0 + i as f32 * 32.0, 26.0, WHITE_SOFT);
        }
        ui.center(tip, 860.0, 24.0, GREY);
        ui.center("Entrée ou R : rejouer  ·  Échap : menu principal", 1020.0, 28.0, GOLD);
        if let Some(replay) = &self.worst_miss {
            let scale = Ui::scale();
            let cx = screen_width() - 150.0 * scale;
            let cy = 430.0 * scale;
            aim::AimTest::draw_replay(replay, cx, cy, 78.0 * scale);
            self.ui.center_at("Pire raté", cx, 430.0 - 115.0, 22.0, GOLD);
            self.ui.center_at(&format!("{:.1}° à côté", replay.error), cx, 430.0 + 115.0, 20.0, GREY);
        }
        if stats.impacts.len() >= 3 {
            let scale = Ui::scale();
            let cx = 150.0 * scale;
            let cy = 430.0 * scale;
            aim::AimTest::draw_cloud(&stats.impacts, cx, cy, 78.0 * scale);
            self.ui.center_at("Nuage des tirs", cx, 430.0 - 115.0, 22.0, GOLD);
        }
    }

    fn draw_options(&self) {
        dim(0.78);
        let ui = &self.ui;
        ui.center("OPTIONS", 220.0, 70.0, ACCENT);
        let volume = format!("{} %", self.volume);
        let rows = [("Axe vertical", if self.invert_y { "Inversé" } else { "Normal" }), ("Volume des sons", volume.as_str())];
        let left = screen_width() / 2.0 - 360.0 * Ui::scale();
        let right = screen_width() / 2.0 + 40.0 * Ui::scale();
        for (i, (label, value)) in rows.iter().enumerate() {
            let y = 420.0 + i as f32 * 80.0;
            let selected = i == self.options_row;
            if selected {
                draw_rectangle(left - 24.0 * Ui::scale(), (y - 48.0) * Ui::scale(), 820.0 * Ui::scale(), 64.0 * Ui::scale(), Color::new(0.0, 1.0, 0.78, 0.08));
            }
            let color = if selected { WHITE_SOFT } else { GREY };
            ui.text(label, left, y, 32.0, color);
            ui.text(&format!("<  {value}  >"), right, y, 32.0, if selected { ACCENT } else { color });
        }
        ui.center(
            "Inversé : pousser la souris vers l'avant regarde vers le bas. Le volume se fait entendre tout de suite.",
            620.0,
            26.0,
            GREY,
        );
        ui.center("↑ ↓ : réglage  ·  ← → : modifier  ·  Entrée ou Échap : retour", 1020.0, 26.0, GOLD);
    }

    fn daily_scores(&self) -> (Option<f32>, Option<f32>) {
        let offset = win::local_offset_secs();
        let day = |time: f64| (time as i64 + offset).div_euclid(86400);
        let today = day(miniquad::date::now());
        let mut yesterday = None;
        let mut best = None;
        for entry in self.history.iter().filter(|entry| entry.mode == 9) {
            best = Some(best.map_or(entry.score, |score: f32| score.max(entry.score)));
            if day(entry.time) == today - 1 {
                yesterday = Some(yesterday.map_or(entry.score, |score: f32| score.max(entry.score)));
            }
        }
        (yesterday, best)
    }

    fn draw_transfer(&self) {
        dim(0.82);
        let ui = &self.ui;
        ui.center("TRANSFERT DANS LE JEU", 140.0, 56.0, ACCENT);
        ui.center(
            &format!("Vitesse du pointeur Windows : {}/20", self.pointer_speed),
            210.0,
            28.0,
            if (9..=11).contains(&self.pointer_speed) { ACCENT } else { GOLD },
        );
        let accel = if self.pointer_accel {
            "Accélération Windows encore activée : décochez « Améliorer la précision du pointeur »."
        } else {
            "Accélération Windows désactivée."
        };
        ui.center(accel, 250.0, 24.0, if self.pointer_accel { RED } else { GREY });

        let left = screen_width() / 2.0 - 520.0 * Ui::scale();
        for (i, step) in TRANSFER_STEPS.iter().enumerate() {
            let y = 360.0 + i as f32 * 80.0;
            let on = (self.transfer & (1 << i)) != 0;
            let selected = i == self.transfer_row;
            if selected {
                draw_rectangle(left - 24.0 * Ui::scale(), (y - 48.0) * Ui::scale(), 1080.0 * Ui::scale(), 64.0 * Ui::scale(), Color::new(0.0, 1.0, 0.78, 0.08));
            }
            let mark = if on { "[x]" } else { "[ ]" };
            let color = if on { ACCENT } else if selected { WHITE_SOFT } else { GREY };
            ui.text(&format!("{}  {}", i + 1, mark), left, y, 32.0, color);
            ui.text(step, left + 140.0 * Ui::scale(), y, 30.0, color);
        }
        ui.center("1 à 4, ou espace : cocher  ·  ↑ ↓ : choisir  ·  Entrée : retour au résultat", 980.0, 26.0, GOLD);
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

fn draw_samples(samples: &[f32], left: f32, base: f32, width: f32, height: f32) {
    if samples.len() < 2 {
        return;
    }
    let min = samples.iter().copied().fold(f32::MAX, f32::min);
    let max = samples.iter().copied().fold(f32::MIN, f32::max);
    let span = (max - min).max(0.5);
    let x = |i: usize| left + width * i as f32 / (samples.len() - 1) as f32;
    let y = |value: f32| base - ((value - min) / span) * height;
    for i in 0..samples.len() - 1 {
        draw_line(x(i), y(samples[i]), x(i + 1), y(samples[i + 1]), 3.0, ACCENT);
        draw_circle(x(i), y(samples[i]), 4.0, WHITE_SOFT);
    }
    let last = samples.len() - 1;
    draw_circle(x(last), y(samples[last]), 4.0, WHITE_SOFT);
}

fn menu_rect(item: usize, count: usize) -> Rect {
    let s = Ui::scale();
    let h = if count > 9 { 58.0 } else if count > 6 { 72.0 } else { 92.0 };
    let step = h + if count > 9 { 6.0 } else { 10.0 };
    let start = if count > 9 { 188.0 } else { 230.0 };
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
    app.sfx.set_volume(app.volume);
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

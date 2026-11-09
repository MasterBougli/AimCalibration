/// Jeu supporté, avec son « yaw » : degrés tournés par compte (count) de souris à sensibilité 1.
pub struct Game {
    pub name: &'static str,
    pub yaw: f32,
    pub settings: &'static str,
}

pub const GAMES: &[Game] = &[
    Game {
        name: "Valorant",
        yaw: 0.07,
        settings: "Paramètres > Général > Souris > Sensibilité : visée",
    },
    Game {
        name: "Counter-Strike 2",
        yaw: 0.022,
        settings: "Paramètres > Clavier / souris > Sensibilité de la souris",
    },
    Game {
        name: "Apex Legends",
        yaw: 0.022,
        settings: "Paramètres > Souris / clavier > Sensibilité de la souris",
    },
    Game {
        name: "Overwatch 2",
        yaw: 0.0066,
        settings: "Options > Commandes > Général > Sensibilité",
    },
    Game {
        name: "Call of Duty (MW / Warzone)",
        yaw: 0.0066,
        settings: "Paramètres > Manette / souris > Souris > Sensibilité de la souris",
    },
    Game {
        name: "Team Fortress 2 / Quake",
        yaw: 0.022,
        settings: "Options > Souris > Sensibilité de la souris",
    },
];

/// Point de départ quand le joueur ne connaît pas sa sensibilité.
pub const DEFAULT_CM_PER_360: f32 = 35.0;

/// 360° × 2,54 cm par pouce.
const DEG_CM_PER_INCH: f32 = 914.4;

pub fn cm_per_360(yaw: f32, sens: f32, dpi: f32) -> f32 {
    DEG_CM_PER_INCH / (yaw * sens * dpi)
}

pub fn sens_for(yaw: f32, cm_per_360: f32, dpi: f32) -> f32 {
    DEG_CM_PER_INCH / (yaw * cm_per_360 * dpi)
}

pub fn deg_per_count(cm_per_360: f32, dpi: f32) -> f32 {
    DEG_CM_PER_INCH / (cm_per_360 * dpi)
}

const COMMON_DPI: &[f32] = &[400.0, 450.0, 500.0, 800.0, 1000.0, 1200.0, 1600.0, 2000.0, 3200.0, 6400.0];

/// Valeur de DPI standard la plus probable pour une mesure approximative.
pub fn likely_dpi(measured: f32) -> f32 {
    let rel_error = |dpi: f32| (measured - dpi).abs() / dpi;
    COMMON_DPI
        .iter()
        .copied()
        .min_by(|a, b| rel_error(*a).total_cmp(&rel_error(*b)))
        .filter(|dpi| rel_error(*dpi) <= 0.08)
        .unwrap_or_else(|| (measured / 50.0).round() * 50.0)
}

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;

// =====================
// Enums
// =====================
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AmmoKind {
    Practice,
    He,
    Smoke,
    Flare,
}

impl AmmoKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AmmoKind::Practice => "PRACTICE",
            AmmoKind::He => "HE",
            AmmoKind::Smoke => "SMOKE",
            AmmoKind::Flare => "FLARE",
        }
    }

    pub fn all() -> &'static [AmmoKind] {
        &[AmmoKind::Practice, AmmoKind::He, AmmoKind::Smoke, AmmoKind::Flare]
    }

    pub fn from_str(s: &str) -> Option<AmmoKind> {
        match s.to_uppercase().as_str() {
            "PRACTICE" => Some(AmmoKind::Practice),
            "HE" => Some(AmmoKind::He),
            "SMOKE" => Some(AmmoKind::Smoke),
            "FLARE" => Some(AmmoKind::Flare),
            _ => None,
        }
    }
}

impl std::fmt::Display for AmmoKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TargetType {
    #[default]
    Infanterie,
    Vehicule,
    Soutien,
}

impl TargetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetType::Infanterie => "INFANTERIE",
            TargetType::Vehicule => "VEHICULE",
            TargetType::Soutien => "SOUTIEN",
        }
    }

    pub fn all() -> &'static [TargetType] {
        &[TargetType::Infanterie, TargetType::Vehicule, TargetType::Soutien]
    }

    pub fn from_str(s: &str) -> Option<TargetType> {
        match s.to_uppercase().as_str() {
            "INFANTERIE" | "INF" => Some(TargetType::Infanterie),
            "VEHICULE" | "VEH" => Some(TargetType::Vehicule),
            "SOUTIEN" | "SOU" => Some(TargetType::Soutien),
            _ => None,
        }
    }

    /// Suggested ammo for this target type
    pub fn suggested_ammo(&self) -> AmmoKind {
        match self {
            TargetType::Infanterie => AmmoKind::He,
            TargetType::Vehicule => AmmoKind::He,
            TargetType::Soutien => AmmoKind::Smoke,
        }
    }
}

impl std::fmt::Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =====================
// Geometry structs
// =====================
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub name: String,
    pub elevation: f64,
    pub x: f64,
    pub y: f64,
}

impl Position {
    pub fn new(name: String, elevation: f64, x: f64, y: f64) -> Self {
        Position { name, elevation, x, y }
    }

    pub fn distance_to(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn elevation_difference(&self, other: &Position) -> f64 {
        (self.elevation - other.elevation).abs()
    }

    pub fn azimuth_to(&self, other: &Position) -> f64 {
        let dy = other.y - self.y;
        let dx = other.x - self.x;
        let mut azimuth = dx.atan2(dy).to_degrees();
        if azimuth < 0.0 {
            azimuth += 360.0;
        }
        azimuth
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MortarPosition {
    pub name: String,
    pub elevation: f64,
    pub x: f64,
    pub y: f64,
    pub ammo_type: AmmoKind,
}

impl MortarPosition {
    pub fn new(name: String, elevation: f64, x: f64, y: f64, ammo_type: AmmoKind) -> Self {
        MortarPosition { name, elevation, x, y, ammo_type }
    }

    pub fn as_position(&self) -> Position {
        Position::new(self.name.clone(), self.elevation, self.x, self.y)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TargetPosition {
    pub name: String,
    pub elevation: f64,
    pub x: f64,
    pub y: f64,
    pub target_type: TargetType,
}

impl TargetPosition {
    pub fn new(name: String, elevation: f64, x: f64, y: f64, target_type: TargetType) -> Self {
        TargetPosition { name, elevation, x, y, target_type }
    }

    pub fn as_position(&self) -> Position {
        Position::new(self.name.clone(), self.elevation, self.x, self.y)
    }
}

// =====================
// Ballistics
// =====================
#[derive(Clone, Debug)]
pub struct BallisticPoint {
    pub range_m: f64,
    pub elev_mil: f64,
}

#[derive(Clone, Debug)]
pub struct BallisticTable {
    pub points: Vec<BallisticPoint>,
}

impl BallisticTable {
    pub fn from_csv<P: AsRef<Path>>(path: P) -> Result<Self> {
        #[derive(Deserialize)]
        struct Row {
            range_m: f64,
            elev_mil: f64,
        }

        let f = File::open(&path)?;
        let mut rdr = csv::Reader::from_reader(f);

        let mut pts: Vec<BallisticPoint> = Vec::new();
        for rec in rdr.deserialize::<Row>() {
            let r = rec?;
            if r.range_m.is_finite() && r.elev_mil.is_finite() {
                pts.push(BallisticPoint { range_m: r.range_m, elev_mil: r.elev_mil });
            }
        }

        pts.sort_by(|a, b| a.range_m.partial_cmp(&b.range_m).unwrap());
        Ok(Self { points: pts })
    }

    pub fn range_bounds(&self) -> Option<(f64, f64)> {
        let first = self.points.first()?.range_m;
        let last = self.points.last()?.range_m;
        Some((first, last))
    }

    pub fn elev_at(&self, range_m: f64) -> Option<f64> {
        if self.points.len() < 2 {
            return None;
        }
        let (minr, maxr) = self.range_bounds()?;
        if range_m < minr || range_m > maxr {
            return None;
        }

        if let Ok(i) = self.points.binary_search_by(|p| p.range_m.partial_cmp(&range_m).unwrap()) {
            return Some(self.points[i].elev_mil);
        }

        let idx = match self.points.binary_search_by(|p| p.range_m.partial_cmp(&range_m).unwrap()) {
            Ok(i) => i,
            Err(ins) => ins.saturating_sub(1),
        };
        if idx + 1 >= self.points.len() {
            return Some(self.points.last()?.elev_mil);
        }

        let p0 = &self.points[idx];
        let p1 = &self.points[idx + 1];
        let t = (range_m - p0.range_m) / (p1.range_m - p0.range_m);
        Some(p0.elev_mil + t * (p1.elev_mil - p0.elev_mil))
    }
}

pub type Ring = u8;

pub fn load_ballistics() -> Result<BTreeMap<(AmmoKind, Ring), BallisticTable>> {
    load_ballistics_from("data")
}

pub fn load_ballistics_from<P: AsRef<Path>>(base: P) -> Result<BTreeMap<(AmmoKind, Ring), BallisticTable>> {
    let base = base.as_ref();
    let mut m: BTreeMap<(AmmoKind, Ring), BallisticTable> = BTreeMap::new();

    // PRACTICE (0..4)
    for r in 0..=4u8 {
        let p = base.join(format!("PRACTICE/M879_PRACTICE_{}R.csv", r));
        if let Ok(t) = BallisticTable::from_csv(&p) {
            m.insert((AmmoKind::Practice, r), t);
        }
    }

    // HE (0..4)
    for r in 0..=4u8 {
        let p = base.join(format!("HE/M821_HE_{}R.csv", r));
        if let Ok(t) = BallisticTable::from_csv(&p) {
            m.insert((AmmoKind::He, r), t);
        }
    }

    // SMOKE (1..4)
    for r in 1..=4u8 {
        let p = base.join(format!("SMOKE/M819_SMOKE_{}R.csv", r));
        if let Ok(t) = BallisticTable::from_csv(&p) {
            m.insert((AmmoKind::Smoke, r), t);
        }
    }

    // FLARE (1..4)
    for r in 1..=4u8 {
        let p = base.join(format!("FLARE/M853A1_FLARE_{}R.csv", r));
        if let Ok(t) = BallisticTable::from_csv(&p) {
            m.insert((AmmoKind::Flare, r), t);
        }
    }

    Ok(m)
}

// =====================
// Solution calculation
// =====================
#[derive(Clone, Debug, Serialize)]
pub struct FiringSolution {
    pub distance_m: f64,
    pub azimuth_deg: f64,
    pub elevation_diff_m: f64,
    pub mortar_ammo: String,
    pub target_type: String,
    pub recommended_ammo: String,
    pub solutions: BTreeMap<String, BTreeMap<String, Option<f64>>>,
    pub selected_solution: Option<SelectedSolution>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SelectedSolution {
    pub ammo_type: String,
    pub elevations: BTreeMap<String, Option<f64>>,
}

pub fn calculate_solution(
    mortar: &MortarPosition,
    target: &TargetPosition,
    ballistics: &BTreeMap<(AmmoKind, Ring), BallisticTable>,
) -> FiringSolution {
    let mortar_pos = mortar.as_position();
    let target_pos = target.as_position();

    let distance_m = mortar_pos.distance_to(&target_pos);
    let azimuth_deg = mortar_pos.azimuth_to(&target_pos);
    let elevation_diff_m = mortar_pos.elevation_difference(&target_pos);

    let rings: &[u8] = &[0, 1, 2, 3, 4];
    let kinds = AmmoKind::all();

    let mut solutions: BTreeMap<String, BTreeMap<String, Option<f64>>> = BTreeMap::new();

    for kind in kinds {
        let mut ring_solutions: BTreeMap<String, Option<f64>> = BTreeMap::new();
        for r in rings {
            let key = format!("{}R", r);
            let elev = ballistics.get(&(*kind, *r)).and_then(|t| t.elev_at(distance_m));
            ring_solutions.insert(key, elev);
        }
        solutions.insert(kind.as_str().to_string(), ring_solutions);
    }

    // Selected solution based on mortar's ammo type
    let selected_ammo = mortar.ammo_type;
    let mut selected_elevations: BTreeMap<String, Option<f64>> = BTreeMap::new();
    for r in rings {
        let key = format!("{}R", r);
        let elev = ballistics.get(&(selected_ammo, *r)).and_then(|t| t.elev_at(distance_m));
        selected_elevations.insert(key, elev);
    }

    let selected_solution = Some(SelectedSolution {
        ammo_type: selected_ammo.as_str().to_string(),
        elevations: selected_elevations,
    });

    FiringSolution {
        distance_m,
        azimuth_deg,
        elevation_diff_m,
        mortar_ammo: mortar.ammo_type.as_str().to_string(),
        target_type: target.target_type.as_str().to_string(),
        recommended_ammo: target.target_type.suggested_ammo().as_str().to_string(),
        solutions,
        selected_solution,
    }
}

// Legacy function for backward compatibility
pub fn calculate_solution_simple(
    mortar: &Position,
    target: &Position,
    ballistics: &BTreeMap<(AmmoKind, Ring), BallisticTable>,
) -> FiringSolution {
    let mortar_pos = MortarPosition::new(
        mortar.name.clone(),
        mortar.elevation,
        mortar.x,
        mortar.y,
        AmmoKind::He,
    );
    let target_pos = TargetPosition::new(
        target.name.clone(),
        target.elevation,
        target.x,
        target.y,
        TargetType::Infanterie,
    );
    calculate_solution(&mortar_pos, &target_pos, ballistics)
}

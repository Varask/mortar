//! # Mortar - Calculateur Balistique pour Mortier 60mm
//!
//! Cette bibliothèque fournit les fonctionnalités de calcul balistique pour un système
//! de mortier 60mm. Elle permet de calculer des solutions de tir en fonction des positions
//! du mortier et de la cible, avec prise en compte des tables balistiques et des dispersions.
//!
//! ## Fonctionnalités principales
//!
//! - Calcul de distance, azimut et élévation entre mortier et cible
//! - Support de 4 types de munitions : Practice (M879), HE (M821), Smoke (M819), Flare (M853A1)
//! - Système d'anneaux de précision (0R à 4R)
//! - Calcul de dispersion ajustée selon le dénivelé
//! - Correction de tir basée sur les déviations observées
//!
//! ## Exemple d'utilisation
//!
//! ```rust,ignore
//! use mortar::{MortarPosition, TargetPosition, AmmoKind, TargetType};
//! use mortar::{load_ballistics, load_dispersion, calculate_solution_with_dispersion};
//!
//! // Charger les tables balistiques et de dispersion
//! let ballistics = load_ballistics().unwrap();
//! let dispersions = load_dispersion().unwrap();
//!
//! // Définir les positions
//! let mortar = MortarPosition::new("M1".to_string(), 100.0, 0.0, 0.0, AmmoKind::He);
//! let target = TargetPosition::new("T1".to_string(), 50.0, 500.0, 300.0, TargetType::Infanterie);
//!
//! // Calculer la solution de tir
//! let solution = calculate_solution_with_dispersion(&mortar, &target, &ballistics, &dispersions);
//!
//! println!("Distance: {:.1} m", solution.distance_m);
//! println!("Azimut: {:.1} deg", solution.azimuth_deg);
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

// ============================================================================
// Types de munitions
// ============================================================================

/// Type de munition disponible pour le mortier 60mm.
///
/// Chaque type de munition a des caractéristiques balistiques différentes
/// et est adapté à des usages spécifiques.
///
/// # Variantes
///
/// - `Practice` - Munition d'entraînement M879 (anneaux 0R-4R)
/// - `He` - Munition explosive M821 High Explosive (anneaux 0R-4R)
/// - `Smoke` - Munition fumigène M819 (anneaux 1R-4R, pas de 0R)
/// - `Flare` - Munition éclairante M853A1 (anneaux 1R-4R, pas de 0R)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AmmoKind {
    /// Munition d'entraînement M879
    Practice,
    /// Munition explosive M821 (High Explosive)
    He,
    /// Munition fumigène M819
    Smoke,
    /// Munition éclairante M853A1
    Flare,
}

impl AmmoKind {
    /// Retourne la représentation textuelle du type de munition.
    ///
    /// # Exemple
    ///
    /// ```
    /// use mortar::AmmoKind;
    /// assert_eq!(AmmoKind::He.as_str(), "HE");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            AmmoKind::Practice => "PRACTICE",
            AmmoKind::He => "HE",
            AmmoKind::Smoke => "SMOKE",
            AmmoKind::Flare => "FLARE",
        }
    }

    /// Retourne un slice contenant tous les types de munitions disponibles.
    ///
    /// # Exemple
    ///
    /// ```
    /// use mortar::AmmoKind;
    /// assert_eq!(AmmoKind::all().len(), 4);
    /// ```
    pub fn all() -> &'static [AmmoKind] {
        &[AmmoKind::Practice, AmmoKind::He, AmmoKind::Smoke, AmmoKind::Flare]
    }

    /// Parse une chaîne de caractères en type de munition.
    ///
    /// La conversion est insensible à la casse.
    ///
    /// # Arguments
    ///
    /// * `s` - Chaîne à parser ("PRACTICE", "HE", "SMOKE", "FLARE")
    ///
    /// # Retourne
    ///
    /// `Some(AmmoKind)` si la chaîne est valide, `None` sinon.
    ///
    /// # Exemple
    ///
    /// ```
    /// use mortar::AmmoKind;
    /// assert_eq!(AmmoKind::from_str("he"), Some(AmmoKind::He));
    /// assert_eq!(AmmoKind::from_str("invalid"), None);
    /// ```
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

// ============================================================================
// Types de cibles
// ============================================================================

/// Type de cible pour la classification tactique.
///
/// Le type de cible influence la munition recommandée pour l'engagement.
///
/// # Variantes
///
/// - `Infanterie` - Personnel à découvert ou en position (recommandation: HE)
/// - `Vehicule` - Véhicules légers non blindés (recommandation: HE)
/// - `Soutien` - Position de soutien, marquage, écran (recommandation: SMOKE)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TargetType {
    /// Cible d'infanterie - Personnel ennemi
    #[default]
    Infanterie,
    /// Cible véhicule - Véhicules légers
    Vehicule,
    /// Cible de soutien - Marquage, écran fumigène
    Soutien,
}

impl TargetType {
    /// Retourne la représentation textuelle du type de cible.
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetType::Infanterie => "INFANTERIE",
            TargetType::Vehicule => "VEHICULE",
            TargetType::Soutien => "SOUTIEN",
        }
    }

    /// Retourne un slice contenant tous les types de cibles disponibles.
    pub fn all() -> &'static [TargetType] {
        &[TargetType::Infanterie, TargetType::Vehicule, TargetType::Soutien]
    }

    /// Parse une chaîne de caractères en type de cible.
    ///
    /// Accepte les formes complètes et abrégées (INF, VEH, SOU).
    ///
    /// # Arguments
    ///
    /// * `s` - Chaîne à parser
    ///
    /// # Exemple
    ///
    /// ```
    /// use mortar::TargetType;
    /// assert_eq!(TargetType::from_str("INF"), Some(TargetType::Infanterie));
    /// assert_eq!(TargetType::from_str("vehicule"), Some(TargetType::Vehicule));
    /// ```
    pub fn from_str(s: &str) -> Option<TargetType> {
        match s.to_uppercase().as_str() {
            "INFANTERIE" | "INF" => Some(TargetType::Infanterie),
            "VEHICULE" | "VEH" => Some(TargetType::Vehicule),
            "SOUTIEN" | "SOU" => Some(TargetType::Soutien),
            _ => None,
        }
    }

    /// Retourne le type de munition suggéré pour ce type de cible.
    ///
    /// # Recommandations
    ///
    /// - Infanterie → HE (effet de fragmentation)
    /// - Véhicule → HE (effet de souffle et fragmentation)
    /// - Soutien → SMOKE (écran fumigène)
    ///
    /// # Exemple
    ///
    /// ```
    /// use mortar::{TargetType, AmmoKind};
    /// assert_eq!(TargetType::Infanterie.suggested_ammo(), AmmoKind::He);
    /// assert_eq!(TargetType::Soutien.suggested_ammo(), AmmoKind::Smoke);
    /// ```
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

// ============================================================================
// Structures géométriques
// ============================================================================

/// Position générique dans un système de coordonnées 2D avec élévation.
///
/// Utilisé comme base pour les positions de mortier et de cible.
///
/// # Système de coordonnées
///
/// - `x` : Coordonnée Est-Ouest (positif vers l'Est)
/// - `y` : Coordonnée Nord-Sud (positif vers le Nord)
/// - `elevation` : Altitude en mètres au-dessus du niveau de référence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    /// Identifiant de la position
    pub name: String,
    /// Altitude en mètres
    pub elevation: f64,
    /// Coordonnée X (Est-Ouest) en mètres
    pub x: f64,
    /// Coordonnée Y (Nord-Sud) en mètres
    pub y: f64,
}

impl Position {
    /// Crée une nouvelle position.
    ///
    /// # Arguments
    ///
    /// * `name` - Identifiant de la position
    /// * `elevation` - Altitude en mètres
    /// * `x` - Coordonnée X en mètres
    /// * `y` - Coordonnée Y en mètres
    pub fn new(name: String, elevation: f64, x: f64, y: f64) -> Self {
        Position { name, elevation, x, y }
    }

    /// Calcule la distance horizontale (2D) vers une autre position.
    ///
    /// Ne prend pas en compte la différence d'altitude.
    ///
    /// # Arguments
    ///
    /// * `other` - Position cible
    ///
    /// # Retourne
    ///
    /// Distance en mètres.
    ///
    /// # Exemple
    ///
    /// ```
    /// use mortar::Position;
    /// let p1 = Position::new("A".to_string(), 0.0, 0.0, 0.0);
    /// let p2 = Position::new("B".to_string(), 0.0, 300.0, 400.0);
    /// assert_eq!(p1.distance_to(&p2), 500.0); // 3-4-5 triangle
    /// ```
    pub fn distance_to(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Calcule la différence d'élévation absolue avec une autre position.
    ///
    /// # Arguments
    ///
    /// * `other` - Position cible
    ///
    /// # Retourne
    ///
    /// Différence d'altitude en mètres (toujours positive).
    pub fn elevation_difference(&self, other: &Position) -> f64 {
        (self.elevation - other.elevation).abs()
    }

    /// Calcule l'azimut vers une autre position.
    ///
    /// L'azimut est mesuré en degrés dans le sens horaire depuis le Nord.
    ///
    /// # Arguments
    ///
    /// * `other` - Position cible
    ///
    /// # Retourne
    ///
    /// Azimut en degrés (0-360).
    ///
    /// # Exemple
    ///
    /// ```
    /// use mortar::Position;
    /// let p1 = Position::new("A".to_string(), 0.0, 0.0, 0.0);
    /// let p2 = Position::new("B".to_string(), 0.0, 100.0, 0.0); // Est
    /// assert!((p1.azimuth_to(&p2) - 90.0).abs() < 0.01);
    /// ```
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

/// Position d'un mortier avec son type de munition chargée.
///
/// Étend la position de base avec le type de munition actuellement
/// configuré pour le mortier.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MortarPosition {
    /// Identifiant du mortier (ex: "M1", "Alpha")
    pub name: String,
    /// Altitude en mètres
    pub elevation: f64,
    /// Coordonnée X en mètres
    pub x: f64,
    /// Coordonnée Y en mètres
    pub y: f64,
    /// Type de munition chargée
    pub ammo_type: AmmoKind,
}

impl MortarPosition {
    /// Crée une nouvelle position de mortier.
    ///
    /// # Arguments
    ///
    /// * `name` - Identifiant du mortier
    /// * `elevation` - Altitude en mètres
    /// * `x` - Coordonnée X en mètres
    /// * `y` - Coordonnée Y en mètres
    /// * `ammo_type` - Type de munition chargée
    pub fn new(name: String, elevation: f64, x: f64, y: f64, ammo_type: AmmoKind) -> Self {
        MortarPosition { name, elevation, x, y, ammo_type }
    }

    /// Convertit en position générique (perd l'information de munition).
    pub fn as_position(&self) -> Position {
        Position::new(self.name.clone(), self.elevation, self.x, self.y)
    }
}

/// Position d'une cible avec son type tactique.
///
/// Étend la position de base avec le type de cible pour
/// déterminer la munition recommandée.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TargetPosition {
    /// Identifiant de la cible (ex: "T1", "Objectif Alpha")
    pub name: String,
    /// Altitude en mètres
    pub elevation: f64,
    /// Coordonnée X en mètres
    pub x: f64,
    /// Coordonnée Y en mètres
    pub y: f64,
    /// Type tactique de la cible
    pub target_type: TargetType,
}

impl TargetPosition {
    /// Crée une nouvelle position de cible.
    ///
    /// # Arguments
    ///
    /// * `name` - Identifiant de la cible
    /// * `elevation` - Altitude en mètres
    /// * `x` - Coordonnée X en mètres
    /// * `y` - Coordonnée Y en mètres
    /// * `target_type` - Type tactique de la cible
    pub fn new(name: String, elevation: f64, x: f64, y: f64, target_type: TargetType) -> Self {
        TargetPosition { name, elevation, x, y, target_type }
    }

    /// Convertit en position générique (perd l'information de type).
    pub fn as_position(&self) -> Position {
        Position::new(self.name.clone(), self.elevation, self.x, self.y)
    }
}

// ============================================================================
// Tables balistiques
// ============================================================================

/// Point de données balistiques associant une portée à une élévation.
///
/// Représente un point de la table de tir pour une munition et un anneau donnés.
#[derive(Clone, Debug)]
pub struct BallisticPoint {
    /// Portée en mètres
    pub range_m: f64,
    /// Élévation en millièmes (mils)
    pub elev_mil: f64,
}

/// Table balistique contenant les points de données pour une munition/anneau.
///
/// Permet l'interpolation linéaire pour obtenir l'élévation à n'importe
/// quelle portée dans les limites de la table.
#[derive(Clone, Debug)]
pub struct BallisticTable {
    /// Points de données triés par portée croissante
    pub points: Vec<BallisticPoint>,
}

impl BallisticTable {
    /// Charge une table balistique depuis un fichier CSV.
    ///
    /// Le fichier doit contenir au minimum les colonnes `range_m` et `elev_mil`.
    ///
    /// # Arguments
    ///
    /// * `path` - Chemin vers le fichier CSV
    ///
    /// # Erreurs
    ///
    /// Retourne une erreur si le fichier ne peut pas être lu ou parsé.
    ///
    /// # Format CSV attendu
    ///
    /// ```csv
    /// range_m,elev_mil,time_flight_s,delta_elev_per_100m_mil,time_flight_per_100m_s
    /// 50,1540,13.2,61,
    /// 100,1479,13.2,63,0.2
    /// ```
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

    /// Retourne les bornes de portée de la table (min, max).
    ///
    /// # Retourne
    ///
    /// `Some((min, max))` si la table contient des points, `None` sinon.
    pub fn range_bounds(&self) -> Option<(f64, f64)> {
        let first = self.points.first()?.range_m;
        let last = self.points.last()?.range_m;
        Some((first, last))
    }

    /// Calcule l'élévation pour une portée donnée par interpolation linéaire.
    ///
    /// # Arguments
    ///
    /// * `range_m` - Portée en mètres
    ///
    /// # Retourne
    ///
    /// `Some(elev_mil)` si la portée est dans les limites de la table, `None` sinon.
    ///
    /// # Algorithme
    ///
    /// Utilise une interpolation linéaire entre les deux points encadrant
    /// la portée demandée.
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

/// Type alias pour le numéro d'anneau de précision (0-4).
pub type Ring = u8;

// ============================================================================
// Données de dispersion
// ============================================================================

/// Structure interne pour la désérialisation du fichier metrics.json.
#[derive(Clone, Debug, Deserialize)]
pub struct MetricsFile {
    /// Map des dispersions par type de munition et anneau
    pub dispersion: BTreeMap<String, BTreeMap<String, f64>>,
}

/// Table de dispersion associant chaque couple (munition, anneau) à un rayon de dispersion.
///
/// Les valeurs sont en mètres et représentent le rayon de dispersion probable
/// (CEP - Circular Error Probable).
pub type DispersionTable = BTreeMap<(AmmoKind, Ring), f64>;

/// Charge les données de dispersion depuis le répertoire par défaut (`data/`).
///
/// # Erreurs
///
/// Retourne une erreur si le fichier `data/metrics.json` ne peut pas être lu.
pub fn load_dispersion() -> Result<DispersionTable> {
    load_dispersion_from("data")
}

/// Charge les données de dispersion depuis un répertoire spécifié.
///
/// # Arguments
///
/// * `base` - Chemin du répertoire contenant `metrics.json`
///
/// # Format du fichier metrics.json
///
/// ```json
/// {
///     "dispersion": {
///         "HE": { "0R": 10, "1R": 23, "2R": 39, "3R": 54, "4R": 69 },
///         "PRACTICE": { "0R": 10, "1R": 24, "2R": 39, "3R": 54, "4R": 68 }
///     }
/// }
/// ```
pub fn load_dispersion_from<P: AsRef<Path>>(base: P) -> Result<DispersionTable> {
    let path = base.as_ref().join("metrics.json");
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let metrics: MetricsFile = serde_json::from_reader(reader)?;

    let mut table = DispersionTable::new();

    for (ammo_str, rings) in &metrics.dispersion {
        let ammo = match AmmoKind::from_str(ammo_str) {
            Some(a) => a,
            None => continue,
        };

        for (ring_str, &value) in rings {
            let ring: Ring = ring_str
                .trim_end_matches('R')
                .parse()
                .unwrap_or(0);
            table.insert((ammo, ring), value);
        }
    }

    Ok(table)
}

/// Calcule la dispersion ajustée en fonction du dénivelé mortier-cible.
///
/// La dispersion est modifiée selon la règle suivante :
/// - Mortier plus haut que la cible : **+5% par mètre** de dénivelé
/// - Mortier plus bas que la cible : **-1% par mètre** de dénivelé
///
/// # Arguments
///
/// * `base_dispersion` - Dispersion de base en mètres (depuis la table)
/// * `mortar_elevation` - Altitude du mortier en mètres
/// * `target_elevation` - Altitude de la cible en mètres
///
/// # Retourne
///
/// Dispersion ajustée en mètres.
///
/// # Formule
///
/// ```text
/// delta = mortar_elevation - target_elevation
/// si delta >= 0 : dispersion = base * (1 + delta * 0.05)
/// si delta < 0  : dispersion = base * (1 + delta * 0.01)
/// ```
///
/// # Exemple
///
/// ```
/// use mortar::calculate_dispersion;
///
/// // Mortier 5m plus haut que la cible, dispersion base 39m
/// let disp = calculate_dispersion(39.0, 105.0, 100.0);
/// assert!((disp - 48.75).abs() < 0.01); // 39 * 1.25 = 48.75
///
/// // Mortier 10m plus bas que la cible
/// let disp = calculate_dispersion(39.0, 90.0, 100.0);
/// assert!((disp - 35.1).abs() < 0.01); // 39 * 0.90 = 35.1
/// ```
pub fn calculate_dispersion(
    base_dispersion: f64,
    mortar_elevation: f64,
    target_elevation: f64,
) -> f64 {
    let delta = mortar_elevation - target_elevation;
    let factor = if delta >= 0.0 {
        1.0 + delta * 0.05  // +5% per meter when mortar is higher
    } else {
        1.0 + delta * 0.01  // -1% per meter when mortar is lower (delta is negative)
    };
    base_dispersion * factor
}

// ============================================================================
// Chargement des tables balistiques
// ============================================================================

/// Charge toutes les tables balistiques depuis le répertoire par défaut (`data/`).
///
/// # Erreurs
///
/// Retourne une erreur si les fichiers CSV ne peuvent pas être lus.
pub fn load_ballistics() -> Result<BTreeMap<(AmmoKind, Ring), BallisticTable>> {
    load_ballistics_from("data")
}

/// Charge toutes les tables balistiques depuis un répertoire spécifié.
///
/// # Structure attendue
///
/// ```text
/// base/
/// ├── PRACTICE/
/// │   ├── M879_PRACTICE_0R.csv
/// │   ├── M879_PRACTICE_1R.csv
/// │   └── ...
/// ├── HE/
/// │   ├── M821_HE_0R.csv
/// │   └── ...
/// ├── SMOKE/
/// │   ├── M819_SMOKE_1R.csv  (pas de 0R pour SMOKE)
/// │   └── ...
/// └── FLARE/
///     ├── M853A1_FLARE_1R.csv  (pas de 0R pour FLARE)
///     └── ...
/// ```
///
/// # Arguments
///
/// * `base` - Chemin du répertoire de données
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

    // SMOKE (1..4) - pas de 0R
    for r in 1..=4u8 {
        let p = base.join(format!("SMOKE/M819_SMOKE_{}R.csv", r));
        if let Ok(t) = BallisticTable::from_csv(&p) {
            m.insert((AmmoKind::Smoke, r), t);
        }
    }

    // FLARE (1..4) - pas de 0R
    for r in 1..=4u8 {
        let p = base.join(format!("FLARE/M853A1_FLARE_{}R.csv", r));
        if let Ok(t) = BallisticTable::from_csv(&p) {
            m.insert((AmmoKind::Flare, r), t);
        }
    }

    Ok(m)
}

// ============================================================================
// Solution de tir
// ============================================================================

/// Solution de tir complète entre un mortier et une cible.
///
/// Contient toutes les informations nécessaires pour effectuer un tir :
/// - Paramètres géométriques (distance, azimut, dénivelé)
/// - Élévations pour chaque type de munition et anneau
/// - Dispersions ajustées pour le dénivelé
/// - Solution sélectionnée basée sur la munition du mortier
#[derive(Clone, Debug, Serialize)]
pub struct FiringSolution {
    /// Distance horizontale en mètres
    pub distance_m: f64,
    /// Azimut en degrés (0-360, depuis le Nord)
    pub azimuth_deg: f64,
    /// Différence d'élévation absolue en mètres
    pub elevation_diff_m: f64,
    /// Différence d'élévation signée (mortier - cible, positif = mortier plus haut)
    pub signed_elevation_diff_m: f64,
    /// Type de munition chargée dans le mortier
    pub mortar_ammo: String,
    /// Type tactique de la cible
    pub target_type: String,
    /// Type de munition recommandé pour cette cible
    pub recommended_ammo: String,
    /// Élévations par type de munition et anneau
    /// Structure: `{ "HE": { "0R": 1200.5, "1R": 1180.2, ... }, ... }`
    pub solutions: BTreeMap<String, BTreeMap<String, Option<f64>>>,
    /// Dispersions ajustées par type de munition et anneau (en mètres)
    pub dispersions: BTreeMap<String, BTreeMap<String, Option<f64>>>,
    /// Solution sélectionnée basée sur la munition du mortier
    pub selected_solution: Option<SelectedSolution>,
}

/// Solution de tir sélectionnée pour un type de munition spécifique.
#[derive(Clone, Debug, Serialize)]
pub struct SelectedSolution {
    /// Type de munition
    pub ammo_type: String,
    /// Élévations par anneau (en mils)
    pub elevations: BTreeMap<String, Option<f64>>,
    /// Dispersions ajustées par anneau (en mètres)
    pub dispersions: BTreeMap<String, Option<f64>>,
}

/// Calcule la solution de tir sans données de dispersion.
///
/// Équivalent à `calculate_solution_with_dispersion` avec une table de dispersion vide.
///
/// # Arguments
///
/// * `mortar` - Position du mortier
/// * `target` - Position de la cible
/// * `ballistics` - Tables balistiques chargées
pub fn calculate_solution(
    mortar: &MortarPosition,
    target: &TargetPosition,
    ballistics: &BTreeMap<(AmmoKind, Ring), BallisticTable>,
) -> FiringSolution {
    calculate_solution_with_dispersion(mortar, target, ballistics, &DispersionTable::new())
}

/// Calcule la solution de tir complète avec dispersions ajustées.
///
/// # Arguments
///
/// * `mortar` - Position du mortier avec type de munition
/// * `target` - Position de la cible avec type tactique
/// * `ballistics` - Tables balistiques chargées
/// * `dispersion_table` - Table des dispersions de base
///
/// # Retourne
///
/// Une [`FiringSolution`] contenant toutes les informations de tir.
///
/// # Exemple
///
/// ```rust,ignore
/// let solution = calculate_solution_with_dispersion(&mortar, &target, &ballistics, &dispersions);
///
/// println!("Distance: {} m", solution.distance_m);
/// println!("Azimut: {} deg", solution.azimuth_deg);
///
/// if let Some(sel) = &solution.selected_solution {
///     println!("Élévation 2R: {:?} mils", sel.elevations.get("2R"));
///     println!("Dispersion 2R: {:?} m", sel.dispersions.get("2R"));
/// }
/// ```
pub fn calculate_solution_with_dispersion(
    mortar: &MortarPosition,
    target: &TargetPosition,
    ballistics: &BTreeMap<(AmmoKind, Ring), BallisticTable>,
    dispersion_table: &DispersionTable,
) -> FiringSolution {
    let mortar_pos = mortar.as_position();
    let target_pos = target.as_position();

    let distance_m = mortar_pos.distance_to(&target_pos);
    let azimuth_deg = mortar_pos.azimuth_to(&target_pos);
    let elevation_diff_m = mortar_pos.elevation_difference(&target_pos);
    let signed_elevation_diff_m = mortar.elevation - target.elevation;

    let rings: &[u8] = &[0, 1, 2, 3, 4];
    let kinds = AmmoKind::all();

    let mut solutions: BTreeMap<String, BTreeMap<String, Option<f64>>> = BTreeMap::new();
    let mut dispersions: BTreeMap<String, BTreeMap<String, Option<f64>>> = BTreeMap::new();

    for kind in kinds {
        let mut ring_solutions: BTreeMap<String, Option<f64>> = BTreeMap::new();
        let mut ring_dispersions: BTreeMap<String, Option<f64>> = BTreeMap::new();
        for r in rings {
            let key = format!("{}R", r);
            let elev = ballistics.get(&(*kind, *r)).and_then(|t| t.elev_at(distance_m));
            ring_solutions.insert(key.clone(), elev);

            let disp = dispersion_table.get(&(*kind, *r)).map(|&base| {
                calculate_dispersion(base, mortar.elevation, target.elevation)
            });
            ring_dispersions.insert(key, disp);
        }
        solutions.insert(kind.as_str().to_string(), ring_solutions);
        dispersions.insert(kind.as_str().to_string(), ring_dispersions);
    }

    // Selected solution based on mortar's ammo type
    let selected_ammo = mortar.ammo_type;
    let mut selected_elevations: BTreeMap<String, Option<f64>> = BTreeMap::new();
    let mut selected_dispersions: BTreeMap<String, Option<f64>> = BTreeMap::new();
    for r in rings {
        let key = format!("{}R", r);
        let elev = ballistics.get(&(selected_ammo, *r)).and_then(|t| t.elev_at(distance_m));
        selected_elevations.insert(key.clone(), elev);

        let disp = dispersion_table.get(&(selected_ammo, *r)).map(|&base| {
            calculate_dispersion(base, mortar.elevation, target.elevation)
        });
        selected_dispersions.insert(key, disp);
    }

    let selected_solution = Some(SelectedSolution {
        ammo_type: selected_ammo.as_str().to_string(),
        elevations: selected_elevations,
        dispersions: selected_dispersions,
    });

    FiringSolution {
        distance_m,
        azimuth_deg,
        elevation_diff_m,
        signed_elevation_diff_m,
        mortar_ammo: mortar.ammo_type.as_str().to_string(),
        target_type: target.target_type.as_str().to_string(),
        recommended_ammo: target.target_type.suggested_ammo().as_str().to_string(),
        solutions,
        dispersions,
        selected_solution,
    }
}

// ============================================================================
// Correction de tir
// ============================================================================

/// Applique une correction à une position de cible basée sur la déviation observée.
///
/// Lorsqu'un tir dévie de sa cible, cette fonction calcule la nouvelle position
/// corrigée de la cible pour compenser la déviation.
///
/// # Convention de signes
///
/// - `vertical_m` : Nord (négatif) / Sud (positif)
/// - `horizontal_m` : Ouest (négatif) / Est (positif)
///
/// # Arguments
///
/// * `target` - Position de cible originale
/// * `vertical_m` - Déviation verticale observée en mètres
/// * `horizontal_m` - Déviation horizontale observée en mètres
///
/// # Retourne
///
/// Une nouvelle [`TargetPosition`] avec les coordonnées corrigées.
/// Le nom de la cible corrigée est suffixé par `_C`.
///
/// # Exemple
///
/// ```
/// use mortar::{TargetPosition, TargetType, apply_correction};
///
/// let target = TargetPosition::new("T1".to_string(), 100.0, 500.0, 300.0, TargetType::Infanterie);
///
/// // L'obus est tombé 50m au Nord et 30m à l'Est de la cible
/// let corrected = apply_correction(&target, -50.0, 30.0);
///
/// assert_eq!(corrected.name, "T1_C");
/// assert_eq!(corrected.x, 470.0);  // 500 - 30 (compense vers l'Ouest)
/// assert_eq!(corrected.y, 350.0);  // 300 - (-50) (compense vers le Sud)
/// ```
pub fn apply_correction(
    target: &TargetPosition,
    vertical_m: f64,
    horizontal_m: f64,
) -> TargetPosition {
    // Correction is the opposite of the deviation
    // If shell landed North of target (negative vertical), we need to move target South (add to Y)
    // If shell landed East of target (positive horizontal), we need to move target West (subtract from X)
    let corrected_x = target.x - horizontal_m;
    let corrected_y = target.y - vertical_m;

    let corrected_name = if target.name.ends_with("_C") {
        target.name.clone()
    } else {
        format!("{}_C", target.name)
    };

    TargetPosition::new(
        corrected_name,
        target.elevation,
        corrected_x,
        corrected_y,
        target.target_type,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn ammo_kind_roundtrip() {
        for &k in AmmoKind::all() {
            let s = k.as_str();
            let parsed = AmmoKind::from_str(s).expect("parse should succeed");
            assert_eq!(parsed, k);
        }
    }

    #[test]
    fn target_type_roundtrip_and_suggested_ammo() {
        for &t in TargetType::all() {
            let s = t.as_str();
            let parsed = TargetType::from_str(s).expect("parse should succeed");
            assert_eq!(parsed, t);
        }
        assert_eq!(TargetType::Infanterie.suggested_ammo(), AmmoKind::He);
        assert_eq!(TargetType::Vehicule.suggested_ammo(), AmmoKind::He);
        assert_eq!(TargetType::Soutien.suggested_ammo(), AmmoKind::Smoke);
    }

    #[test]
    fn position_examples_hold() {
        let p1 = Position::new("A".to_string(), 0.0, 0.0, 0.0);
        let p2 = Position::new("B".to_string(), 0.0, 300.0, 400.0);
        assert_eq!(p1.distance_to(&p2), 500.0);

        let east = Position::new("E".to_string(), 0.0, 100.0, 0.0);
        let az = p1.azimuth_to(&east);
        assert!((az - 90.0).abs() < 0.01);
    }

    #[test]
    fn calculate_dispersion_matches_doc() {
        let d1 = calculate_dispersion(39.0, 105.0, 100.0);
        assert!((d1 - 48.75).abs() < 0.01);

        let d2 = calculate_dispersion(39.0, 90.0, 100.0);
        assert!((d2 - 35.1).abs() < 0.01);
    }

    #[test]
    fn ballistic_table_interpolation_and_bounds() {
        let table = BallisticTable {
            points: vec![
                BallisticPoint { range_m: 0.0, elev_mil: 1000.0 },
                BallisticPoint { range_m: 100.0, elev_mil: 900.0 },
            ],
        };

        assert_eq!(table.elev_at(0.0), Some(1000.0));
        assert_eq!(table.elev_at(100.0), Some(900.0));
        let mid = table.elev_at(50.0).unwrap();
        assert!((mid - 950.0).abs() < 1e-6);
        assert_eq!(table.elev_at(-10.0), None);
        assert_eq!(table.elev_at(150.0), None);
    }

    #[test]
    fn apply_correction_example() {
        let t = TargetPosition::new(
            "T1".to_string(),
            100.0,
            500.0,
            300.0,
            TargetType::Infanterie,
        );

        let corrected = apply_correction(&t, -50.0, 30.0);

        assert_eq!(corrected.name, "T1_C");
        assert_eq!(corrected.x, 470.0);
        assert_eq!(corrected.y, 350.0);
    }

    #[test]
    fn calculate_solution_with_dispersion_populates_struct() {
        let mut ballistics: BTreeMap<(AmmoKind, Ring), BallisticTable> = BTreeMap::new();
        ballistics.insert(
            (AmmoKind::He, 2),
            BallisticTable {
                points: vec![
                    BallisticPoint { range_m: 0.0, elev_mil: 1200.0 },
                    BallisticPoint { range_m: 600.0, elev_mil: 1100.0 },
                ],
            },
        );
        let mut dispersions: DispersionTable = BTreeMap::new();
        dispersions.insert((AmmoKind::He, 2), 39.0);

        let mortar = MortarPosition::new("M1".into(), 100.0, 0.0, 0.0, AmmoKind::He);
        let target = TargetPosition::new("T1".into(), 50.0, 500.0, 300.0, TargetType::Infanterie);

        let sol = calculate_solution_with_dispersion(&mortar, &target, &ballistics, &dispersions);

        assert!(sol.distance_m > 0.0);
        assert!(sol.azimuth_deg >= 0.0 && sol.azimuth_deg <= 360.0);
        assert_eq!(sol.mortar_ammo, "HE");
        assert_eq!(sol.target_type, "INFANTERIE");
        assert_eq!(sol.recommended_ammo, "HE");
        assert!(sol.solutions.get("HE").is_some());
        assert!(sol.dispersions.get("HE").is_some());
        let sel = sol.selected_solution.as_ref().expect("selected_solution");
        assert_eq!(sel.ammo_type, "HE");
        assert!(sel.elevations.contains_key("2R"));
        assert!(sel.dispersions.contains_key("2R"));
    }
}

pub mod pchip;
pub mod server;
pub mod server_cli;

// Re-export so server_cli can `use crate::AppState;`
pub use server::AppState;

use rustyline::completion::Completer;
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::validate::Validator;

use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};

// =====================
// Autocomplete helper
// =====================
struct CommandHelper {
    commands: Vec<&'static str>,
}

impl Completer for CommandHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<String>)> {
        let start = line.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let prefix = &line[start..];

        let matches: Vec<String> = self
            .commands
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|cmd| cmd.to_string())
            .collect();

        Ok((start, matches))
    }
}

impl Hinter for CommandHelper {
    type Hint = String;
}
impl Highlighter for CommandHelper {}
impl Validator for CommandHelper {}
impl rustyline::Helper for CommandHelper {}

// =====================
// UI Functions
// =====================

fn clear_screen() {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "cls"])
            .status()
            .ok();
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("clear")
            .status()
            .ok();
    }
}

// =====================
// Geometry structs
// =====================
struct Position {
    name: String,
    elevation: f64,
    x: f64,
    y: f64,
}

impl Position {
    fn new(name: String, elevation: f64, x: f64, y: f64) -> Self {
        Position { name, elevation, x, y }
    }

    fn distance_to(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    fn elevation_difference(&self, other: &Position) -> f64 {
        (self.elevation - other.elevation).abs()
    }

    fn azumuth_to(&self, other: &Position) -> f64 {
        let dy = other.y - self.y;
        let dx = other.x - self.x;
        let mut azimuth = dx.atan2(dy).to_degrees();
        if azimuth < 0.0 {
            azimuth += 360.0;
        }
        azimuth
    }
}

struct Mortars {
    mortar_pos: Vec<Position>,
    target_pos: Vec<Position>,
}

impl Mortars {
    fn new() -> Self {
        Mortars { mortar_pos: Vec::new(), target_pos: Vec::new() }
    }

    fn add_mortar(&mut self, position: Position) {
        self.mortar_pos.push(position);
    }

    fn add_target(&mut self, position: Position) {
        self.target_pos.push(position);
    }
}

// =====================
// Ballistics
// =====================
#[derive(Clone, Debug)]
struct BallisticPoint {
    range_m: f64,
    elev_mil: f64,
}

#[derive(Clone, Debug)]
struct BallisticTable {
    points: Vec<BallisticPoint>, // triés par range
}

impl BallisticTable {
    fn from_csv<P: AsRef<Path>>(path: P) -> Result<Self> {
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

    fn range_bounds(&self) -> Option<(f64, f64)> {
        let first = self.points.first()?.range_m;
        let last = self.points.last()?.range_m;
        Some((first, last))
    }

    // interpolation linéaire (stable). Retourne None si hors plage.
    fn elev_at(&self, range_m: f64) -> Option<f64> {
        if self.points.len() < 2 {
            return None;
        }
        let (minr, maxr) = self.range_bounds()?;
        if range_m < minr || range_m > maxr {
            return None;
        }

        // exact match
        if let Ok(i) = self.points.binary_search_by(|p| p.range_m.partial_cmp(&range_m).unwrap()) {
            return Some(self.points[i].elev_mil);
        }

        // find segment
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum AmmoKind {
    Practice,
    He,
    Smoke,
    Flare,
}

impl AmmoKind {
    fn as_str(&self) -> &'static str {
        match self {
            AmmoKind::Practice => "PRACTICE",
            AmmoKind::He => "HE",
            AmmoKind::Smoke => "SMOKE",
            AmmoKind::Flare => "FLARE",
        }
    }
}

type Ring = u8;

fn load_ballistics() -> Result<BTreeMap<(AmmoKind, Ring), BallisticTable>> {
    let mut m: BTreeMap<(AmmoKind, Ring), BallisticTable> = BTreeMap::new();

    // Adapte les chemins si besoin:
    // data/HE/M821_HE_4R.csv etc.

    // PRACTICE (0..4)
    for r in 0..=4u8 {
        let p = format!("data/PRACTICE/M879_PRACTICE_{}R.csv", r);
        if let Ok(t) = BallisticTable::from_csv(&p) {
            m.insert((AmmoKind::Practice, r), t);
        }
    }

    // HE (0..4)
    for r in 0..=4u8 {
        let p = format!("data/HE/M821_HE_{}R.csv", r);
        if let Ok(t) = BallisticTable::from_csv(&p) {
            m.insert((AmmoKind::He, r), t);
        }
    }

    // SMOKE (1..4)
    for r in 1..=4u8 {
        let p = format!("data/SMOKE/M819_SMOKE_{}R.csv", r);
        if let Ok(t) = BallisticTable::from_csv(&p) {
            m.insert((AmmoKind::Smoke, r), t);
        }
    }

    // FLARE (1..4)
    for r in 1..=4u8 {
        let p = format!("data/FLARE/M853A1_FLARE_{}R.csv", r);
        if let Ok(t) = BallisticTable::from_csv(&p) {
            m.insert((AmmoKind::Flare, r), t);
        }
    }

    Ok(m)
}

fn print_solution_table(ball: &BTreeMap<(AmmoKind, Ring), BallisticTable>, distance_m: f64) {
    let rings: &[u8] = &[0, 1, 2, 3, 4];
    let kinds: &[AmmoKind] = &[AmmoKind::Practice, AmmoKind::He, AmmoKind::Smoke, AmmoKind::Flare];

    println!("\n--- Elevation (mil) @ {:.2} m ---", distance_m);

    // header
    print!("{:>10} |", "TYPE");
    for r in rings {
        print!(" {:>7} |", format!("{}R", r));
    }
    println!();
    println!("{}", "-".repeat(10 + 2 + rings.len() * 10));

    // rows
    for k in kinds {
        print!("{:>10} |", k.as_str());
        for r in rings {
            let v = ball.get(&(*k, *r)).and_then(|t| t.elev_at(distance_m));
            match v {
                Some(e) => print!(" {:>7.1} |", e),
                None => print!(" {:>7} |", "N/A"),
            }
        }
        println!();
    }
    println!();
}

// =====================
// CLI loop
// =====================
fn main() {
    let mut mortars = Mortars::new();

    let ballistics = load_ballistics().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load ballistics: {e}");
        BTreeMap::new()
    });

    wait_for_command(&mut mortars, ballistics);
}

fn wait_for_command(mortars: &mut Mortars, ballistics: BTreeMap<(AmmoKind, Ring), BallisticTable>) {
    use rustyline::error::ReadlineError;
    use rustyline::history::DefaultHistory;
    use rustyline::{Config, Editor};

    let config = Config::builder().build();
    let mut rl: Editor<CommandHelper, DefaultHistory> =
        Editor::with_config(config).expect("Failed to create editor");

    let commands = vec![
        "add_mortar",
        "add_target",
        "calculate",
        "rm_mortar",
        "rm_target",
        "list",
        "clear",
        "help",
        "exit",
        "adjust"
    ];

    let helper = CommandHelper { commands };
    rl.set_helper(Some(helper));
    let _ = rl.load_history(".mortar_history");

    loop {
        match rl.readline("> ") {
            Ok(line) => {
                let _ = rl.add_history_entry(&line);
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                match parts[0] {
                    "add_mortar" => add_mortar(mortars, &parts),
                    "add_target" => add_target(mortars, &parts),
                    "calculate" => calculate(mortars, &parts, &ballistics),
                    "rm_mortar" => rm_mortar(mortars, &parts),
                    "rm_target" => rm_target(mortars, &parts),
                    "list" => list(mortars),
                    "clear" => clear_screen(),
                    "help" => show_help(&parts),
                    "exit" => break,
                    _ => println!("Commande inconnue: '{}'. Tapez 'help' pour l'aide", parts[0]),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("exit");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    let _ = rl.save_history(".mortar_history");
}

// =====================
// Commands
// =====================
fn add_mortar(mortars: &mut Mortars, args: &[&str]) {
    if args.len() < 5 {
        println!("Usage: add_mortar <name> <elevation> <x> <y>");
        return;
    }

    let name = args[1].to_string();
    let elevation = match args[2].parse::<f64>() {
        Ok(e) => e,
        Err(_) => { println!("Erreur: elevation invalide"); return; }
    };
    let x = match args[3].parse::<f64>() {
        Ok(x) => x,
        Err(_) => { println!("Erreur: x invalide"); return; }
    };
    let y = match args[4].parse::<f64>() {
        Ok(y) => y,
        Err(_) => { println!("Erreur: y invalide"); return; }
    };

    mortars.add_mortar(Position::new(name.clone(), elevation, x, y));
    println!("Mortier '{}' ajouté", name);
}

fn add_target(mortars: &mut Mortars, args: &[&str]) {
    if args.len() < 5 {
        println!("Usage: add_target <name> <elevation> <x> <y>");
        return;
    }

    let name = args[1].to_string();
    let elevation = match args[2].parse::<f64>() {
        Ok(e) => e,
        Err(_) => { println!("Erreur: elevation invalide"); return; }
    };
    let x = match args[3].parse::<f64>() {
        Ok(x) => x,
        Err(_) => { println!("Erreur: x invalide"); return; }
    };
    let y = match args[4].parse::<f64>() {
        Ok(y) => y,
        Err(_) => { println!("Erreur: y invalide"); return; }
    };

    mortars.add_target(Position::new(name.clone(), elevation, x, y));
    println!("Cible '{}' ajoutée", name);
}

fn calculate(mortars: &Mortars, args: &[&str], ballistics: &BTreeMap<(AmmoKind, Ring), BallisticTable>) {
    if args.len() < 3 {
        println!("Usage: calculate <mortar_name> <target_name>");
        return;
    }

    let mortar_name = args[1];
    let target_name = args[2];

    let mortar = mortars.mortar_pos.iter().find(|m| m.name == mortar_name);
    let target = mortars.target_pos.iter().find(|t| t.name == target_name);

    match (mortar, target) {
        (Some(m), Some(t)) => {
            let distance = m.distance_to(t);
            let elevation_diff = m.elevation_difference(t);
            let azimuth = m.azumuth_to(t);

            println!("Solution de tir:");
            println!("  Distance: {:.2} m", distance);
            println!("  Différence d'élévation: {:.2} m", elevation_diff);
            println!("  Azimut: {:.2}°", azimuth);

            // Tableau élévation par munition/rings (N/A si hors plage ou CSV manquant)
            if !ballistics.is_empty() {
                print_solution_table(ballistics, distance);
            } else {
                println!("(Ballistics non chargées: aucun tableau affiché)");
            }
        }
        _ => println!("Mortier ou cible non trouvé"),
    }
}

fn rm_mortar(mortars: &mut Mortars, args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: rm_mortar <name>");
        return;
    }
    let name = args[1];
    mortars.mortar_pos.retain(|m| m.name != name);
    println!("Mortier '{}' supprimé", name);
}

fn rm_target(mortars: &mut Mortars, args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: rm_target <name>");
        return;
    }
    let name = args[1];
    mortars.target_pos.retain(|t| t.name != name);
    println!("Cible '{}' supprimée", name);
}

fn list(mortars: &Mortars) {
    println!("\n--- Mortiers ---");
    for mortar in &mortars.mortar_pos {
        println!("  {}: x={}, y={}, élévation={}", mortar.name, mortar.x, mortar.y, mortar.elevation);
    }

    println!("\n--- Cibles ---");
    for target in &mortars.target_pos {
        println!("  {}: x={}, y={}, élévation={}", target.name, target.x, target.y, target.elevation);
    }
    println!();
}

fn show_help(args: &[&str]) {
    if args.len() < 2 {
        println!("\n╔════════════════════════════════════════════════════════════════╗");
        println!("║         CALCULATEUR DE SOLUTION DE TIR - SYSTÈME MORTAR        ║");
        println!("╚════════════════════════════════════════════════════════════════╝\n");

        println!("Commandes disponibles:");
        println!("  add_mortar   - Ajouter un mortier");
        println!("  add_target   - Ajouter une cible");
        println!("  calculate    - Calculer la solution de tir");
        println!("  rm_mortar    - Supprimer un mortier");
        println!("  rm_target    - Supprimer une cible");
        println!("  list         - Afficher les mortiers et cibles");
        println!("  clear        - Effacer l'écran");
        println!("  help         - Afficher cette aide");
        println!("  exit         - Quitter le programme");
        println!("\nPour plus d'infos: help <commande>\n");
    } else {
        match args[1] {
            "add_mortar" => help_add_mortar(),
            "add_target" => help_add_target(),
            "calculate" => help_calculate(),
            "rm_mortar" => help_rm_mortar(),
            "rm_target" => help_rm_target(),
            "list" => help_list(),
            "clear" => help_clear(),
            "exit" => help_exit(),
            _ => println!("Commande '{}' inconnue. Tapez 'help' pour les commandes disponibles", args[1]),
        }
    }
}

// =====================
// Help texts
// =====================
fn help_add_mortar() {
    println!("\n┌─ Commande: add_mortar  ─┐");
    println!("├─────────────────────────┘");
    println!("│ Usage: add_mortar <name> <elevation> <x> <y>");
    println!("│ Exemple: add_mortar m1 100 0 0");
    println!("└─────────────────────────\n");
}

fn help_add_target() {
    println!("\n┌─ Commande: add_target  ─┐");
    println!("├─────────────────────────┘");
    println!("│ Usage: add_target <name> <elevation> <x> <y>");
    println!("│ Exemple: add_target t1 50 100 100");
    println!("└─────────────────────────\n");
}

fn help_calculate() {
    println!("\n┌─ Commande: calculate  ─┐");
    println!("├────────────────────────┘");
    println!("│ Usage: calculate <mortar_name> <target_name>");
    println!("│ Exemple: calculate m1 t1");
    println!("│ Affiche aussi un tableau Elevation(mil) par munition + ring.");
    println!("└────────────────────────\n");
}

fn help_rm_mortar() {
    println!("\n┌─ Commande: rm_mortar  ─┐");
    println!("├────────────────────────┘");
    println!("│ Usage: rm_mortar <name>");
    println!("│ Exemple: rm_mortar m1");
    println!("└────────────────────────\n");
}

fn help_rm_target() {
    println!("\n┌─ Commande: rm_target  ─┐");
    println!("├────────────────────────┘");
    println!("│ Usage: rm_target <name>");
    println!("│ Exemple: rm_target t1");
    println!("└────────────────────────\n");
}

fn help_list() {
    println!("\n┌─ Commande: list  ─┐");
    println!("├────────────────────┘");
    println!("│ Usage: list");
    println!("└────────────────────\n");
}

fn help_clear() {
    println!("\n┌─ Commande: clear ─┐");
    println!("├────────────────────┘");
    println!("│ Description: Efface l'écran du terminal");
    println!("│");
    println!("│ Usage: clear");
    println!("│");
    println!("│ Arguments: (aucun)");
    println!("│");
    println!("│ Note: Fonctionne sur Windows (cls) et Unix/Linux (clear)");
    println!("└────────────────────\n");
}

fn help_exit() {
    println!("\n┌─ Commande: exit  ─┐");
    println!("├────────────────────┘");
    println!("│ Quitte l'application");
    println!("└────────────────────\n");
}

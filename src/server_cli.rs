use crate::{apply_correction, calculate_solution_with_dispersion, AmmoKind, AppState, TargetType};
use std::io::{self, Write};
use std::sync::Arc;

pub async fn handle_cli_command(line: &str, state: &Arc<AppState>) {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    match parts[0] {
        "help" | "h" => print_help(),
        "list" | "ls" => list_all(state).await,

        "add_mortar" | "am" => add_mortar_cli(&parts, state).await,
        "add_target" | "at" => add_target_cli(&parts, state).await,

        "rm_mortar" | "rmm" => rm_mortar_cli(&parts, state).await,
        "rm_target" | "rmt" => rm_target_cli(&parts, state).await,

        "set_ammo" | "sa" => set_ammo_cli(&parts, state).await,
        "set_type" | "st" => set_type_cli(&parts, state).await,

        "calc" | "c" => {
            if parts.len() < 3 {
                println!("Usage: calc <mortar_name> <target_name>");
            } else {
                calc_and_print(state, parts[1], parts[2]).await;
            }
        }

        "correct" | "cor" => {
            if parts.len() < 4 {
                println!("Usage: correct <target_name> <vertical_m> <horizontal_m>");
                println!("  vertical_m:   Nord (negatif) / Sud (positif)");
                println!("  horizontal_m: Ouest (negatif) / Est (positif)");
                println!("  Exemple: correct T1 -50 30  (obus tombe 50m au Nord, 30m a l'Est)");
            } else {
                let target_name = parts[1];
                let vertical: f64 = parts[2].parse().unwrap_or(0.0);
                let horizontal: f64 = parts[3].parse().unwrap_or(0.0);
                correct_target_cli(state, target_name, vertical, horizontal).await;
            }
        }

        "clear" => {
            print!("\x1B[2J\x1B[1;1H");
            let _ = io::stdout().flush();
        }

        _ => println!(
            "Unknown command: '{}'. Type 'help' for available commands.",
            parts[0]
        ),
    }
}

pub fn print_help() {
    println!();
    println!("=== MORTAR CALCULATOR CLI ===");
    println!();
    println!("Commands:");
    println!("  help, h                                    Show this help");
    println!("  list, ls                                   List all mortars and targets");
    println!("  add_mortar, am <n> <e> <x> <y>             Add mortar");
    println!("  add_target, at <n> <e> <x> <y> [type] [ammo]  Add target (type: INF/VEH/SOU, ammo: HE/PRACTICE/SMOKE/FLARE)");
    println!("  rm_mortar, rmm <name>                      Remove mortar");
    println!("  rm_target, rmt <name>                      Remove target");
    println!("  set_ammo, sa <target> <ammo>               Set target ammo type");
    println!("  set_type, st <target> <type>               Set target type");
    println!("  calc, c <mortar> <target>            Calculate firing solution");
    println!("  correct, cor <target> <V> <H>        Correct target position");
    println!("                                         V: Nord(-)/Sud(+)  H: Ouest(-)/Est(+)");
    println!("  clear                                Clear screen");
    println!();
    println!("Web interface available at: http://localhost:3000");
    println!();
}

pub async fn list_all(state: &Arc<AppState>) {
    let mortars = state.mortars.read().await;
    let targets = state.targets.read().await;

    println!();
    println!("--- MORTIERS ({}) ---", mortars.len());
    if mortars.is_empty() {
        println!("  (aucun)");
    } else {
        for m in mortars.iter() {
            println!(
                "  {} : X={:.0} Y={:.0} E={:.0}m",
                m.name, m.x, m.y, m.elevation
            );
        }
    }

    println!();
    println!("--- CIBLES ({}) ---", targets.len());
    if targets.is_empty() {
        println!("  (aucune)");
    } else {
        for t in targets.iter() {
            println!(
                "  {} : X={:.0} Y={:.0} E={:.0}m [{}] [{}]",
                t.name, t.x, t.y, t.elevation, t.target_type, t.ammo_type
            );
        }
    }
    println!();
}

async fn add_mortar_cli(parts: &[&str], state: &Arc<AppState>) {
    if parts.len() < 5 {
        println!("Usage: add_mortar <name> <elevation> <x> <y>");
        return;
    }

    let name = parts[1].to_string();
    let elevation: f64 = parts[2].parse().unwrap_or(0.0);
    let x: f64 = parts[3].parse().unwrap_or(0.0);
    let y: f64 = parts[4].parse().unwrap_or(0.0);

    let mut mortars = state.mortars.write().await;
    if mortars.iter().any(|m| m.name == name) {
        println!("Error: Mortar '{}' already exists", name);
    } else {
        mortars.push(crate::MortarPosition::new(name.clone(), elevation, x, y));
        println!("Mortar '{}' added", name);
    }
}

async fn add_target_cli(parts: &[&str], state: &Arc<AppState>) {
    if parts.len() < 5 {
        println!("Usage: add_target <name> <elevation> <x> <y> [target_type] [ammo_type]");
        println!("  target_type: INFANTERIE/INF, VEHICULE/VEH, SOUTIEN/SOU (default: INFANTERIE)");
        println!("  ammo_type: HE, PRACTICE, SMOKE, FLARE (default: HE)");
        return;
    }

    let name = parts[1].to_string();
    let elevation: f64 = parts[2].parse().unwrap_or(0.0);
    let x: f64 = parts[3].parse().unwrap_or(0.0);
    let y: f64 = parts[4].parse().unwrap_or(0.0);

    let ttype = if parts.len() > 5 {
        TargetType::parse_str(parts[5]).unwrap_or(TargetType::Infanterie)
    } else {
        TargetType::Infanterie
    };

    let ammo = if parts.len() > 6 {
        AmmoKind::parse_str(parts[6]).unwrap_or(AmmoKind::He)
    } else {
        AmmoKind::He
    };

    let mut targets = state.targets.write().await;
    if targets.iter().any(|t| t.name == name) {
        println!("Error: Target '{}' already exists", name);
    } else {
        targets.push(crate::TargetPosition::new(
            name.clone(),
            elevation,
            x,
            y,
            ttype,
            ammo,
        ));
        println!("Target '{}' added as {} [{}]", name, ttype, ammo);
    }
}

async fn rm_mortar_cli(parts: &[&str], state: &Arc<AppState>) {
    if parts.len() < 2 {
        println!("Usage: rm_mortar <name>");
        return;
    }

    let name = parts[1];
    let mut mortars = state.mortars.write().await;
    let before = mortars.len();
    mortars.retain(|m| m.name != name);

    if mortars.len() < before {
        println!("Mortar '{}' deleted", name);
    } else {
        println!("Mortar '{}' not found", name);
    }
}

async fn rm_target_cli(parts: &[&str], state: &Arc<AppState>) {
    if parts.len() < 2 {
        println!("Usage: rm_target <name>");
        return;
    }

    let name = parts[1];
    let mut targets = state.targets.write().await;
    let before = targets.len();
    targets.retain(|t| t.name != name);

    if targets.len() < before {
        println!("Target '{}' deleted", name);
    } else {
        println!("Target '{}' not found", name);
    }
}

async fn set_ammo_cli(parts: &[&str], state: &Arc<AppState>) {
    if parts.len() < 3 {
        println!("Usage: set_ammo <target_name> <ammo_type>");
        println!("  ammo_type: HE, PRACTICE, SMOKE, FLARE");
        return;
    }

    let name = parts[1];
    let ammo = match AmmoKind::parse_str(parts[2]) {
        Some(a) => a,
        None => {
            println!("Invalid ammo type: {}", parts[2]);
            return;
        }
    };

    let mut targets = state.targets.write().await;
    if let Some(t) = targets.iter_mut().find(|t| t.name == name) {
        t.ammo_type = ammo;
        println!("Target '{}' ammo set to {}", name, ammo);
    } else {
        println!("Target '{}' not found", name);
    }
}

async fn set_type_cli(parts: &[&str], state: &Arc<AppState>) {
    if parts.len() < 3 {
        println!("Usage: set_type <target_name> <target_type>");
        println!("  target_type: INFANTERIE/INF, VEHICULE/VEH, SOUTIEN/SOU");
        return;
    }

    let name = parts[1];
    let ttype = match TargetType::parse_str(parts[2]) {
        Some(t) => t,
        None => {
            println!("Invalid target type: {}", parts[2]);
            return;
        }
    };

    let mut targets = state.targets.write().await;
    if let Some(t) = targets.iter_mut().find(|t| t.name == name) {
        t.target_type = ttype;
        println!("Target '{}' type set to {}", name, ttype);
    } else {
        println!("Target '{}' not found", name);
    }
}

pub async fn correct_target_cli(
    state: &Arc<AppState>,
    target_name: &str,
    vertical_m: f64,
    horizontal_m: f64,
) {
    let mut targets = state.targets.write().await;

    let target = match targets.iter().find(|t| t.name == target_name) {
        Some(t) => t.clone(),
        None => {
            println!("Target '{}' not found", target_name);
            return;
        }
    };

    let corrected = apply_correction(&target, vertical_m, horizontal_m);
    let corrected_name = corrected.name.clone();
    let new_x = corrected.x;
    let new_y = corrected.y;

    if let Some(existing) = targets.iter_mut().find(|t| t.name == corrected_name) {
        existing.x = new_x;
        existing.y = new_y;
        println!("Correction mise a jour: {}", corrected_name);
    } else {
        targets.push(corrected);
        println!("Nouvelle cible corrigee: {}", corrected_name);
    }

    println!();
    println!(
        "  Original:  {} -> X={:.0} Y={:.0}",
        target_name, target.x, target.y
    );
    println!(
        "  Deviation: V={:+.0}m (N-/S+) H={:+.0}m (O-/E+)",
        vertical_m, horizontal_m
    );
    println!(
        "  Corrige:   {} -> X={:.0} Y={:.0}",
        corrected_name, new_x, new_y
    );
    println!();
}

pub async fn calc_and_print(state: &Arc<AppState>, mortar_name: &str, target_name: &str) {
    let mortars = state.mortars.read().await;
    let targets = state.targets.read().await;

    let mortar = mortars.iter().find(|m| m.name == mortar_name);
    let target = targets.iter().find(|t| t.name == target_name);

    match (mortar, target) {
        (Some(m), Some(t)) => {
            let solution =
                calculate_solution_with_dispersion(m, t, &state.ballistics, &state.dispersions);

            println!();
            println!("=== SOLUTION DE TIR: {} -> {} ===", m.name, t.name);
            println!();
            println!("  Distance:       {:.1} m", solution.distance_m);
            println!("  Azimut:         {:.1} deg", solution.azimuth_deg);
            println!(
                "  Diff Elevation: {:.1} m (signe: {:+.1} m)",
                solution.elevation_diff_m, solution.signed_elevation_diff_m
            );
            println!();
            println!("  Ogive:          {}", solution.mortar_ammo);
            println!("  Type cible:     {}", solution.target_type);
            println!("  Ogive suggeree: {}", solution.recommended_ammo);
            println!();

            if let Some(sel) = &solution.selected_solution {
                println!("  >>> ELEVATION {} <<<", sel.ammo_type);
                print!("  Elev:");
                for r in 0..=4 {
                    let key = format!("{}R", r);
                    match sel.elevations.get(&key).and_then(|v| *v) {
                        Some(e) => print!(" {}:{:.1}", key, e),
                        None => print!(" {}:N/A", key),
                    }
                }
                println!();
                print!("  Disp:");
                for r in 0..=4 {
                    let key = format!("{}R", r);
                    match sel.dispersions.get(&key).and_then(|v| *v) {
                        Some(d) => print!(" {}:{:.1}m", key, d),
                        None => print!(" {}:N/A", key),
                    }
                }
                println!();
            }

            println!();
            println!("  --- Toutes les elevations (mil) / dispersions (m) ---");
            let rings = ["0R", "1R", "2R", "3R", "4R"];
            print!("  {:>10} |", "TYPE");
            for r in &rings {
                print!(" {:>11} |", r);
            }
            println!();
            println!("  {}", "-".repeat(10 + 2 + rings.len() * 14));

            for ammo in AmmoKind::all() {
                print!("  {:>10} |", ammo.as_str());
                let ammo_sol = solution.solutions.get(ammo.as_str());
                let ammo_disp = solution.dispersions.get(ammo.as_str());

                for r in &rings {
                    let elev = ammo_sol.and_then(|s| s.get(*r).and_then(|v| *v));
                    let disp = ammo_disp.and_then(|d| d.get(*r).and_then(|v| *v));
                    match (elev, disp) {
                        (Some(e), Some(d)) => print!(" {:>5.1}/{:<4.1} |", e, d),
                        (Some(e), None) => print!(" {:>5.1}/---- |", e),
                        (None, _) => print!(" {:>11} |", "N/A"),
                    }
                }
                println!();
            }

            println!();
        }
        (None, _) => println!("Mortar '{}' not found", mortar_name),
        (_, None) => println!("Target '{}' not found", target_name),
    }
}

pub fn print_prompt() {
    print!("> ");
    let _ = io::stdout().flush();
}

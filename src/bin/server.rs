use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use mortar::{
    apply_correction, calculate_solution_with_dispersion, load_ballistics_from, load_dispersion_from,
    AmmoKind, BallisticTable, DispersionTable, FiringSolution, MortarPosition, Ring, TargetPosition, TargetType,
};

// =====================
// Application state
// =====================
struct AppState {
    ballistics: BTreeMap<(AmmoKind, Ring), BallisticTable>,
    dispersions: DispersionTable,
    mortars: RwLock<Vec<MortarPosition>>,
    targets: RwLock<Vec<TargetPosition>>,
}

// =====================
// API types
// =====================
#[derive(Debug, Deserialize)]
struct CalculateByNameRequest {
    mortar_name: String,
    target_name: String,
}

#[derive(Debug, Deserialize)]
struct AddMortarRequest {
    name: String,
    elevation: f64,
    x: f64,
    y: f64,
    #[serde(default = "default_ammo")]
    ammo_type: String,
}

fn default_ammo() -> String {
    "HE".to_string()
}

#[derive(Debug, Deserialize)]
struct AddTargetRequest {
    name: String,
    elevation: f64,
    x: f64,
    y: f64,
    #[serde(default = "default_target_type")]
    target_type: String,
}

fn default_target_type() -> String {
    "INFANTERIE".to_string()
}

#[derive(Debug, Deserialize)]
struct DeletePositionRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct UpdateMortarAmmoRequest {
    name: String,
    ammo_type: String,
}

#[derive(Debug, Deserialize)]
struct UpdateTargetTypeRequest {
    name: String,
    target_type: String,
}

#[derive(Debug, Deserialize)]
struct CorrectionRequest {
    target_name: String,
    vertical_m: f64,   // North (negative) / South (positive)
    horizontal_m: f64, // West (negative) / East (positive)
}

#[derive(Debug, Serialize)]
struct CorrectionResponse {
    success: bool,
    original: String,
    corrected: String,
    correction_applied: CorrectionApplied,
}

#[derive(Debug, Serialize)]
struct CorrectionApplied {
    vertical_m: f64,
    horizontal_m: f64,
    new_x: f64,
    new_y: f64,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct AmmoTypesResponse {
    ammo_types: Vec<AmmoTypeInfo>,
}

#[derive(Debug, Serialize)]
struct AmmoTypeInfo {
    name: String,
    rings: Vec<u8>,
}

#[derive(Debug, Serialize)]
struct MortarListResponse {
    positions: Vec<MortarPosition>,
}

#[derive(Debug, Serialize)]
struct TargetListResponse {
    positions: Vec<TargetPosition>,
}

#[derive(Debug, Serialize)]
struct SuccessResponse {
    success: bool,
    message: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize)]
struct TypesResponse {
    ammo_types: Vec<String>,
    target_types: Vec<String>,
}

// =====================
// Handlers
// =====================
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn get_types() -> Json<TypesResponse> {
    Json(TypesResponse {
        ammo_types: AmmoKind::all().iter().map(|a| a.as_str().to_string()).collect(),
        target_types: TargetType::all().iter().map(|t| t.as_str().to_string()).collect(),
    })
}

async fn get_ammo_types(State(state): State<Arc<AppState>>) -> Json<AmmoTypesResponse> {
    let mut ammo_types = Vec::new();

    for kind in AmmoKind::all() {
        let rings: Vec<u8> = (0..=4)
            .filter(|r| state.ballistics.contains_key(&(*kind, *r)))
            .collect();

        if !rings.is_empty() {
            ammo_types.push(AmmoTypeInfo {
                name: kind.as_str().to_string(),
                rings,
            });
        }
    }

    Json(AmmoTypesResponse { ammo_types })
}

async fn calculate_by_name(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CalculateByNameRequest>,
) -> Result<Json<FiringSolution>, (StatusCode, Json<ErrorResponse>)> {
    let mortars = state.mortars.read().await;
    let targets = state.targets.read().await;

    let mortar = mortars.iter().find(|m| m.name == req.mortar_name);
    let target = targets.iter().find(|t| t.name == req.target_name);

    match (mortar, target) {
        (Some(m), Some(t)) => {
            let solution = calculate_solution_with_dispersion(m, t, &state.ballistics, &state.dispersions);
            Ok(Json(solution))
        }
        (None, _) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Mortar '{}' not found", req.mortar_name),
            }),
        )),
        (_, None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Target '{}' not found", req.target_name),
            }),
        )),
    }
}

// Mortar CRUD
async fn list_mortars(State(state): State<Arc<AppState>>) -> Json<MortarListResponse> {
    let mortars = state.mortars.read().await;
    Json(MortarListResponse {
        positions: mortars.clone(),
    })
}

async fn add_mortar(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddMortarRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    if req.name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Name cannot be empty".to_string(),
            }),
        ));
    }

    let ammo_type = AmmoKind::from_str(&req.ammo_type).unwrap_or(AmmoKind::He);

    let mut mortars = state.mortars.write().await;

    if mortars.iter().any(|m| m.name == req.name) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!("Mortar '{}' already exists", req.name),
            }),
        ));
    }

    mortars.push(MortarPosition::new(req.name.clone(), req.elevation, req.x, req.y, ammo_type));

    Ok(Json(SuccessResponse {
        success: true,
        message: format!("Mortar '{}' added with {}", req.name, ammo_type),
    }))
}

async fn delete_mortar(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeletePositionRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut mortars = state.mortars.write().await;
    let initial_len = mortars.len();
    mortars.retain(|m| m.name != req.name);

    if mortars.len() < initial_len {
        Ok(Json(SuccessResponse {
            success: true,
            message: format!("Mortar '{}' deleted", req.name),
        }))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Mortar '{}' not found", req.name),
            }),
        ))
    }
}

async fn update_mortar_ammo(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateMortarAmmoRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ammo_type = match AmmoKind::from_str(&req.ammo_type) {
        Some(a) => a,
        None => return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid ammo type: {}", req.ammo_type),
            }),
        )),
    };

    let mut mortars = state.mortars.write().await;
    if let Some(mortar) = mortars.iter_mut().find(|m| m.name == req.name) {
        mortar.ammo_type = ammo_type;
        Ok(Json(SuccessResponse {
            success: true,
            message: format!("Mortar '{}' ammo set to {}", req.name, ammo_type),
        }))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Mortar '{}' not found", req.name),
            }),
        ))
    }
}

// Target CRUD
async fn list_targets(State(state): State<Arc<AppState>>) -> Json<TargetListResponse> {
    let targets = state.targets.read().await;
    Json(TargetListResponse {
        positions: targets.clone(),
    })
}

async fn add_target(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddTargetRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    if req.name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Name cannot be empty".to_string(),
            }),
        ));
    }

    let target_type = TargetType::from_str(&req.target_type).unwrap_or(TargetType::Infanterie);

    let mut targets = state.targets.write().await;

    if targets.iter().any(|t| t.name == req.name) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!("Target '{}' already exists", req.name),
            }),
        ));
    }

    targets.push(TargetPosition::new(req.name.clone(), req.elevation, req.x, req.y, target_type));

    Ok(Json(SuccessResponse {
        success: true,
        message: format!("Target '{}' added as {}", req.name, target_type),
    }))
}

async fn delete_target(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeletePositionRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut targets = state.targets.write().await;
    let initial_len = targets.len();
    targets.retain(|t| t.name != req.name);

    if targets.len() < initial_len {
        Ok(Json(SuccessResponse {
            success: true,
            message: format!("Target '{}' deleted", req.name),
        }))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Target '{}' not found", req.name),
            }),
        ))
    }
}

async fn update_target_type(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateTargetTypeRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let target_type = match TargetType::from_str(&req.target_type) {
        Some(t) => t,
        None => return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid target type: {}", req.target_type),
            }),
        )),
    };

    let mut targets = state.targets.write().await;
    if let Some(target) = targets.iter_mut().find(|t| t.name == req.name) {
        target.target_type = target_type;
        Ok(Json(SuccessResponse {
            success: true,
            message: format!("Target '{}' type set to {}", req.name, target_type),
        }))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Target '{}' not found", req.name),
            }),
        ))
    }
}

// Correction handler
async fn correct_target(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CorrectionRequest>,
) -> Result<Json<CorrectionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut targets = state.targets.write().await;

    let target = match targets.iter().find(|t| t.name == req.target_name) {
        Some(t) => t.clone(),
        None => return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Target '{}' not found", req.target_name),
            }),
        )),
    };

    let corrected = apply_correction(&target, req.vertical_m, req.horizontal_m);
    let corrected_name = corrected.name.clone();
    let new_x = corrected.x;
    let new_y = corrected.y;

    // Check if corrected target already exists
    if let Some(existing) = targets.iter_mut().find(|t| t.name == corrected_name) {
        // Update existing corrected target
        existing.x = new_x;
        existing.y = new_y;
    } else {
        // Add new corrected target
        targets.push(corrected);
    }

    Ok(Json(CorrectionResponse {
        success: true,
        original: req.target_name,
        corrected: corrected_name,
        correction_applied: CorrectionApplied {
            vertical_m: req.vertical_m,
            horizontal_m: req.horizontal_m,
            new_x,
            new_y,
        },
    }))
}

// =====================
// CLI Commands
// =====================
async fn handle_cli_command(line: &str, state: &Arc<AppState>) {
    let parts: Vec<&str> = line.trim().split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    match parts[0] {
        "help" | "h" => print_help(),
        "list" | "ls" => list_all(state).await,
        "add_mortar" | "am" => {
            if parts.len() < 5 {
                println!("Usage: add_mortar <name> <elevation> <x> <y> [ammo_type]");
                println!("  ammo_type: HE, PRACTICE, SMOKE, FLARE (default: HE)");
            } else {
                let name = parts[1].to_string();
                let elevation: f64 = parts[2].parse().unwrap_or(0.0);
                let x: f64 = parts[3].parse().unwrap_or(0.0);
                let y: f64 = parts[4].parse().unwrap_or(0.0);
                let ammo = if parts.len() > 5 {
                    AmmoKind::from_str(parts[5]).unwrap_or(AmmoKind::He)
                } else {
                    AmmoKind::He
                };

                let mut mortars = state.mortars.write().await;
                if mortars.iter().any(|m| m.name == name) {
                    println!("Error: Mortar '{}' already exists", name);
                } else {
                    mortars.push(MortarPosition::new(name.clone(), elevation, x, y, ammo));
                    println!("Mortar '{}' added with {} ammo", name, ammo);
                }
            }
        }
        "add_target" | "at" => {
            if parts.len() < 5 {
                println!("Usage: add_target <name> <elevation> <x> <y> [target_type]");
                println!("  target_type: INFANTERIE/INF, VEHICULE/VEH, SOUTIEN/SOU (default: INFANTERIE)");
            } else {
                let name = parts[1].to_string();
                let elevation: f64 = parts[2].parse().unwrap_or(0.0);
                let x: f64 = parts[3].parse().unwrap_or(0.0);
                let y: f64 = parts[4].parse().unwrap_or(0.0);
                let ttype = if parts.len() > 5 {
                    TargetType::from_str(parts[5]).unwrap_or(TargetType::Infanterie)
                } else {
                    TargetType::Infanterie
                };

                let mut targets = state.targets.write().await;
                if targets.iter().any(|t| t.name == name) {
                    println!("Error: Target '{}' already exists", name);
                } else {
                    targets.push(TargetPosition::new(name.clone(), elevation, x, y, ttype));
                    println!("Target '{}' added as {}", name, ttype);
                }
            }
        }
        "rm_mortar" | "rmm" => {
            if parts.len() < 2 {
                println!("Usage: rm_mortar <name>");
            } else {
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
        }
        "rm_target" | "rmt" => {
            if parts.len() < 2 {
                println!("Usage: rm_target <name>");
            } else {
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
        }
        "set_ammo" | "sa" => {
            if parts.len() < 3 {
                println!("Usage: set_ammo <mortar_name> <ammo_type>");
                println!("  ammo_type: HE, PRACTICE, SMOKE, FLARE");
            } else {
                let name = parts[1];
                let ammo = match AmmoKind::from_str(parts[2]) {
                    Some(a) => a,
                    None => {
                        println!("Invalid ammo type: {}", parts[2]);
                        return;
                    }
                };
                let mut mortars = state.mortars.write().await;
                if let Some(m) = mortars.iter_mut().find(|m| m.name == name) {
                    m.ammo_type = ammo;
                    println!("Mortar '{}' ammo set to {}", name, ammo);
                } else {
                    println!("Mortar '{}' not found", name);
                }
            }
        }
        "set_type" | "st" => {
            if parts.len() < 3 {
                println!("Usage: set_type <target_name> <target_type>");
                println!("  target_type: INFANTERIE/INF, VEHICULE/VEH, SOUTIEN/SOU");
            } else {
                let name = parts[1];
                let ttype = match TargetType::from_str(parts[2]) {
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
        }
        "calc" | "c" => {
            if parts.len() < 3 {
                println!("Usage: calc <mortar_name> <target_name>");
            } else {
                let mortar_name = parts[1];
                let target_name = parts[2];
                calc_and_print(state, mortar_name, target_name).await;
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
        _ => println!("Unknown command: '{}'. Type 'help' for available commands.", parts[0]),
    }
}

fn print_help() {
    println!();
    println!("=== MORTAR CALCULATOR CLI ===");
    println!();
    println!("Commands:");
    println!("  help, h                              Show this help");
    println!("  list, ls                             List all mortars and targets");
    println!("  add_mortar, am <n> <e> <x> <y> [ammo]  Add mortar (ammo: HE/PRACTICE/SMOKE/FLARE)");
    println!("  add_target, at <n> <e> <x> <y> [type]  Add target (type: INF/VEH/SOU)");
    println!("  rm_mortar, rmm <name>                Remove mortar");
    println!("  rm_target, rmt <name>                Remove target");
    println!("  set_ammo, sa <mortar> <ammo>         Set mortar ammo type");
    println!("  set_type, st <target> <type>         Set target type");
    println!("  calc, c <mortar> <target>            Calculate firing solution");
    println!("  correct, cor <target> <V> <H>        Correct target position");
    println!("                                         V: Nord(-)/Sud(+)  H: Ouest(-)/Est(+)");
    println!("  clear                                Clear screen");
    println!();
    println!("Web interface available at: http://localhost:3000");
    println!();
}

async fn list_all(state: &Arc<AppState>) {
    let mortars = state.mortars.read().await;
    let targets = state.targets.read().await;

    println!();
    println!("--- MORTIERS ({}) ---", mortars.len());
    if mortars.is_empty() {
        println!("  (aucun)");
    } else {
        for m in mortars.iter() {
            println!("  {} : X={:.0} Y={:.0} E={:.0}m [{}]", m.name, m.x, m.y, m.elevation, m.ammo_type);
        }
    }

    println!();
    println!("--- CIBLES ({}) ---", targets.len());
    if targets.is_empty() {
        println!("  (aucune)");
    } else {
        for t in targets.iter() {
            println!("  {} : X={:.0} Y={:.0} E={:.0}m [{}]", t.name, t.x, t.y, t.elevation, t.target_type);
        }
    }
    println!();
}

async fn correct_target_cli(state: &Arc<AppState>, target_name: &str, vertical_m: f64, horizontal_m: f64) {
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

    // Check if corrected target already exists
    if let Some(existing) = targets.iter_mut().find(|t| t.name == corrected_name) {
        existing.x = new_x;
        existing.y = new_y;
        println!("Correction mise a jour: {}", corrected_name);
    } else {
        targets.push(corrected);
        println!("Nouvelle cible corrigee: {}", corrected_name);
    }

    println!();
    println!("  Original:  {} -> X={:.0} Y={:.0}", target_name, target.x, target.y);
    println!("  Deviation: V={:+.0}m (N-/S+) H={:+.0}m (O-/E+)", vertical_m, horizontal_m);
    println!("  Corrige:   {} -> X={:.0} Y={:.0}", corrected_name, new_x, new_y);
    println!();
}

async fn calc_and_print(state: &Arc<AppState>, mortar_name: &str, target_name: &str) {
    let mortars = state.mortars.read().await;
    let targets = state.targets.read().await;

    let mortar = mortars.iter().find(|m| m.name == mortar_name);
    let target = targets.iter().find(|t| t.name == target_name);

    match (mortar, target) {
        (Some(m), Some(t)) => {
            let solution = calculate_solution_with_dispersion(m, t, &state.ballistics, &state.dispersions);

            println!();
            println!("=== SOLUTION DE TIR: {} -> {} ===", m.name, t.name);
            println!();
            println!("  Distance:       {:.1} m", solution.distance_m);
            println!("  Azimut:         {:.1} deg", solution.azimuth_deg);
            println!("  Diff Elevation: {:.1} m (signe: {:+.1} m)", solution.elevation_diff_m, solution.signed_elevation_diff_m);
            println!();
            println!("  Ogive mortier:  {}", solution.mortar_ammo);
            println!("  Type cible:     {}", solution.target_type);
            println!("  Ogive suggeree: {}", solution.recommended_ammo);
            println!();

            // Print selected solution (based on mortar ammo)
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

// =====================
// Main
// =====================
#[tokio::main]
async fn main() {
    // Determine data path
    let data_path = if std::path::Path::new("data").exists() {
        "data"
    } else if std::path::Path::new("/workspace/rust/mortar/data").exists() {
        "/workspace/rust/mortar/data"
    } else {
        "data"
    };

    println!("Loading ballistics from: {}", data_path);

    let ballistics = load_ballistics_from(data_path).unwrap_or_else(|e| {
        eprintln!("Warning: failed to load ballistics: {e}");
        BTreeMap::new()
    });

    println!("Loaded {} ballistic tables", ballistics.len());

    let dispersions = load_dispersion_from(data_path).unwrap_or_else(|e| {
        eprintln!("Warning: failed to load dispersions: {e}");
        DispersionTable::new()
    });

    println!("Loaded {} dispersion entries", dispersions.len());

    let state = Arc::new(AppState {
        ballistics,
        dispersions,
        mortars: RwLock::new(Vec::new()),
        targets: RwLock::new(Vec::new()),
    });

    // Determine web assets path
    let web_path = if std::path::Path::new("src/web").exists() {
        "src/web"
    } else if std::path::Path::new("/workspace/rust/mortar/src/web").exists() {
        "/workspace/rust/mortar/src/web"
    } else {
        "src/web"
    };

    println!("Serving static files from: {}", web_path);

    // Build router
    let app = Router::new()
        // Health & info
        .route("/api/health", get(health_check))
        .route("/api/types", get(get_types))
        .route("/api/ammo-types", get(get_ammo_types))
        // Calculate
        .route("/api/calculate", post(calculate_by_name))
        // Mortars CRUD
        .route("/api/mortars", get(list_mortars))
        .route("/api/mortars", post(add_mortar))
        .route("/api/mortars", delete(delete_mortar))
        .route("/api/mortars/ammo", post(update_mortar_ammo))
        // Targets CRUD
        .route("/api/targets", get(list_targets))
        .route("/api/targets", post(add_target))
        .route("/api/targets", delete(delete_target))
        .route("/api/targets/type", post(update_target_type))
        .route("/api/targets/correct", post(correct_target))
        // Static files
        .nest_service("/", ServeDir::new(web_path))
        .with_state(state.clone());

    let addr = "0.0.0.0:3000";
    println!("Server starting on http://{}", addr);
    println!();
    println!("Type 'help' for CLI commands");
    println!();

    // Check if running in interactive mode (TTY attached)
    let interactive = atty::is(atty::Stream::Stdin);

    if interactive {
        // Spawn web server in background
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // CLI loop (non-blocking with web server)
        let stdin = io::stdin();
        let reader = stdin.lock();

        print!("> ");
        let _ = io::stdout().flush();

        for line in reader.lines() {
            match line {
                Ok(input) => {
                    if input.trim() == "exit" || input.trim() == "quit" || input.trim() == "q" {
                        println!("Shutting down...");
                        break;
                    }
                    handle_cli_command(&input, &state).await;
                }
                Err(_) => break,
            }
            print!("> ");
            let _ = io::stdout().flush();
        }
    } else {
        // Non-interactive mode (container/daemon): run web server only
        println!("Running in non-interactive mode (web server only)");
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    use mortar::{AmmoKind, BallisticPoint, BallisticTable, Ring};
    use tokio::runtime::Runtime;

    fn test_state_with_ballistics() -> Arc<AppState> {
        let mut ballistics = BTreeMap::new();
        ballistics.insert(
            (AmmoKind::He, 2),
            BallisticTable {
                points: vec![
                    BallisticPoint { range_m: 0.0, elev_mil: 1200.0 },
                    BallisticPoint { range_m: 1000.0, elev_mil: 900.0 },
                ],
            },
        );
        let dispersions = DispersionTable::new();

        Arc::new(AppState {
            ballistics,
            dispersions,
            mortars: RwLock::new(Vec::new()),
            targets: RwLock::new(Vec::new()),
        })
    }

    #[test]
    fn cli_add_mortar_and_target_and_calc_do_not_panic() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let state = test_state_with_ballistics();

            handle_cli_command("am M1 100 0 0 HE", &state).await;
            handle_cli_command("at T1 50 500 300 INFANTERIE", &state).await;
            handle_cli_command("c M1 T1", &state).await;
        });
    }

    #[test]
    fn cli_invalid_command_is_handled() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let state = test_state_with_ballistics();
            handle_cli_command("unknown_cmd", &state).await;
        });
    }
}

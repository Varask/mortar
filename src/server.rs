use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use crate::{
    apply_correction, calculate_solution_with_dispersion, load_ballistics_from,
    load_dispersion_from, AmmoKind, BallisticTable, DispersionTable, FiringSolution,
    MortarPosition, Ring, TargetPosition, TargetType,
};

fn default_ammo() -> String {
    "HE".to_string()
}

// =====================
// Application state
// =====================
pub struct AppState {
    pub ballistics: BTreeMap<(AmmoKind, Ring), BallisticTable>,
    pub dispersions: DispersionTable,
    pub mortars: RwLock<Vec<MortarPosition>>,
    pub targets: RwLock<Vec<TargetPosition>>,
}

// =====================
// API types
// =====================
#[derive(Debug, Deserialize)]
pub struct CalculateByNameRequest {
    pub mortar_name: String,
    pub target_name: String,
}

#[derive(Debug, Deserialize)]
pub struct AddMortarRequest {
    pub name: String,
    pub elevation: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize)]
pub struct AddTargetRequest {
    pub name: String,
    pub elevation: f64,
    pub x: f64,
    pub y: f64,
    #[serde(default = "default_target_type")]
    pub target_type: String,
    #[serde(default = "default_ammo")]
    pub ammo_type: String,
}

fn default_target_type() -> String {
    "INFANTERIE".to_string()
}

#[derive(Debug, Deserialize)]
pub struct DeletePositionRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTargetTypeRequest {
    pub name: String,
    pub target_type: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTargetAmmoRequest {
    pub name: String,
    pub ammo_type: String,
}

#[derive(Debug, Deserialize)]
pub struct CorrectionRequest {
    pub target_name: String,
    pub vertical_m: f64,   // North (negative) / South (positive)
    pub horizontal_m: f64, // West (negative) / East (positive)
}

#[derive(Debug, Serialize)]
pub struct CorrectionResponse {
    pub success: bool,
    pub original: String,
    pub corrected: String,
    pub correction_applied: CorrectionApplied,
}

#[derive(Debug, Serialize)]
pub struct CorrectionApplied {
    pub vertical_m: f64,
    pub horizontal_m: f64,
    pub new_x: f64,
    pub new_y: f64,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct AmmoTypesResponse {
    pub ammo_types: Vec<AmmoTypeInfo>,
}

#[derive(Debug, Serialize)]
pub struct AmmoTypeInfo {
    pub name: String,
    pub rings: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct MortarListResponse {
    pub positions: Vec<MortarPosition>,
}

#[derive(Debug, Serialize)]
pub struct TargetListResponse {
    pub positions: Vec<TargetPosition>,
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct TypesResponse {
    pub ammo_types: Vec<String>,
    pub target_types: Vec<String>,
}

// =====================
// Router builder
// =====================

pub fn build_app_with_state(data_path: &str, web_path: &str) -> (Router, Arc<AppState>) {
    let ballistics = load_ballistics_from(data_path).unwrap_or_else(|e| {
        eprintln!("Warning: failed to load ballistics: {e}");
        BTreeMap::new()
    });

    let dispersions = load_dispersion_from(data_path).unwrap_or_else(|e| {
        eprintln!("Warning: failed to load dispersions: {e}");
        DispersionTable::new()
    });

    let state = Arc::new(AppState {
        ballistics,
        dispersions,
        mortars: RwLock::new(Vec::new()),
        targets: RwLock::new(Vec::new()),
    });

    // IMPORTANT: build as Router<Arc<AppState>> (missing state), then provide it and end as Router<()>.
    let app: Router<Arc<AppState>> = Router::new()
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
        // Targets CRUD
        .route("/api/targets", get(list_targets))
        .route("/api/targets", post(add_target))
        .route("/api/targets", delete(delete_target))
        .route("/api/targets/type", post(update_target_type))
        .route("/api/targets/ammo", post(update_target_ammo))
        .route("/api/targets/correct", post(correct_target))
        // Static files
        .nest_service("/", ServeDir::new(web_path));

    // Provide the Arc<AppState>, choose new “missing state” = () so we return Router (Router<()>).
    let app: Router = app.with_state::<()>(state.clone());

    (app, state)
}

pub fn build_app(data_path: &str, web_path: &str) -> Router {
    build_app_with_state(data_path, web_path).0
}

// =====================
// Handlers
// =====================

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub async fn get_types() -> Json<TypesResponse> {
    Json(TypesResponse {
        ammo_types: AmmoKind::all()
            .iter()
            .map(|a| a.as_str().to_string())
            .collect(),
        target_types: TargetType::all()
            .iter()
            .map(|t| t.as_str().to_string())
            .collect(),
    })
}

pub async fn get_ammo_types(State(state): State<Arc<AppState>>) -> Json<AmmoTypesResponse> {
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

pub async fn calculate_by_name(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CalculateByNameRequest>,
) -> Result<Json<FiringSolution>, (StatusCode, Json<ErrorResponse>)> {
    let mortars = state.mortars.read().await;
    let targets = state.targets.read().await;

    let mortar = mortars.iter().find(|m| m.name == req.mortar_name);
    let target = targets.iter().find(|t| t.name == req.target_name);

    match (mortar, target) {
        (Some(m), Some(t)) => {
            let solution =
                calculate_solution_with_dispersion(m, t, &state.ballistics, &state.dispersions);
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

pub async fn list_mortars(State(state): State<Arc<AppState>>) -> Json<MortarListResponse> {
    let mortars = state.mortars.read().await;
    Json(MortarListResponse {
        positions: mortars.clone(),
    })
}

pub async fn add_mortar(
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

    let mut mortars = state.mortars.write().await;

    if mortars.iter().any(|m| m.name == req.name) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!("Mortar '{}' already exists", req.name),
            }),
        ));
    }

    mortars.push(MortarPosition::new(
        req.name.clone(),
        req.elevation,
        req.x,
        req.y,
    ));

    Ok(Json(SuccessResponse {
        success: true,
        message: format!("Mortar '{}' added", req.name),
    }))
}

pub async fn delete_mortar(
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

pub async fn update_target_ammo(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateTargetAmmoRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ammo_type = match AmmoKind::parse_str(&req.ammo_type) {
        Some(a) => a,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Invalid ammo type: {}", req.ammo_type),
                }),
            ))
        }
    };

    let mut targets = state.targets.write().await;
    if let Some(target) = targets.iter_mut().find(|t| t.name == req.name) {
        target.ammo_type = ammo_type;
        Ok(Json(SuccessResponse {
            success: true,
            message: format!("Target '{}' ammo set to {}", req.name, ammo_type),
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

pub async fn list_targets(State(state): State<Arc<AppState>>) -> Json<TargetListResponse> {
    let targets = state.targets.read().await;
    Json(TargetListResponse {
        positions: targets.clone(),
    })
}

pub async fn add_target(
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

    let target_type = TargetType::parse_str(&req.target_type).unwrap_or(TargetType::Infanterie);
    let ammo_type = AmmoKind::parse_str(&req.ammo_type).unwrap_or(AmmoKind::He);
    let mut targets = state.targets.write().await;

    if targets.iter().any(|t| t.name == req.name) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!("Target '{}' already exists", req.name),
            }),
        ));
    }

    targets.push(TargetPosition::new(
        req.name.clone(),
        req.elevation,
        req.x,
        req.y,
        target_type,
        ammo_type,
    ));

    Ok(Json(SuccessResponse {
        success: true,
        message: format!("Target '{}' added as {}", req.name, target_type),
    }))
}

pub async fn delete_target(
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

pub async fn update_target_type(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateTargetTypeRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let target_type = match TargetType::parse_str(&req.target_type) {
        Some(t) => t,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Invalid target type: {}", req.target_type),
                }),
            ))
        }
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

pub async fn correct_target(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CorrectionRequest>,
) -> Result<Json<CorrectionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut targets = state.targets.write().await;

    let target = match targets.iter().find(|t| t.name == req.target_name) {
        Some(t) => t.clone(),
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Target '{}' not found", req.target_name),
                }),
            ))
        }
    };

    let corrected = apply_correction(&target, req.vertical_m, req.horizontal_m);
    let corrected_name = corrected.name.clone();
    let new_x = corrected.x;
    let new_y = corrected.y;

    if let Some(existing) = targets.iter_mut().find(|t| t.name == corrected_name) {
        existing.x = new_x;
        existing.y = new_y;
    } else {
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

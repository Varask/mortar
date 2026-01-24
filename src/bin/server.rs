use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use mortar::{calculate_solution, load_ballistics_from, AmmoKind, BallisticTable, FiringSolution, Position, Ring};

// =====================
// Application state
// =====================
struct AppState {
    ballistics: BTreeMap<(AmmoKind, Ring), BallisticTable>,
    mortars: RwLock<Vec<Position>>,
    targets: RwLock<Vec<Position>>,
}

// =====================
// API types
// =====================
#[derive(Debug, Deserialize)]
struct CalculateRequest {
    mortar: Position,
    target: Position,
}

#[derive(Debug, Deserialize)]
struct CalculateByNameRequest {
    mortar_name: String,
    target_name: String,
}

#[derive(Debug, Deserialize)]
struct AddPositionRequest {
    name: String,
    elevation: f64,
    x: f64,
    y: f64,
}

#[derive(Debug, Deserialize)]
struct DeletePositionRequest {
    name: String,
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
struct PositionListResponse {
    positions: Vec<Position>,
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

// =====================
// Handlers
// =====================
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
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

// Direct calculation with positions
async fn calculate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CalculateRequest>,
) -> Result<Json<FiringSolution>, (StatusCode, Json<ErrorResponse>)> {
    if req.mortar.name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Mortar name cannot be empty".to_string(),
            }),
        ));
    }

    if req.target.name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Target name cannot be empty".to_string(),
            }),
        ));
    }

    let solution = calculate_solution(&req.mortar, &req.target, &state.ballistics);
    Ok(Json(solution))
}

// Calculate by name (using stored positions)
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
            let solution = calculate_solution(m, t, &state.ballistics);
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
async fn list_mortars(State(state): State<Arc<AppState>>) -> Json<PositionListResponse> {
    let mortars = state.mortars.read().await;
    Json(PositionListResponse {
        positions: mortars.clone(),
    })
}

async fn add_mortar(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddPositionRequest>,
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

    // Check if already exists
    if mortars.iter().any(|m| m.name == req.name) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!("Mortar '{}' already exists", req.name),
            }),
        ));
    }

    mortars.push(Position::new(req.name.clone(), req.elevation, req.x, req.y));

    Ok(Json(SuccessResponse {
        success: true,
        message: format!("Mortar '{}' added", req.name),
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

// Target CRUD
async fn list_targets(State(state): State<Arc<AppState>>) -> Json<PositionListResponse> {
    let targets = state.targets.read().await;
    Json(PositionListResponse {
        positions: targets.clone(),
    })
}

async fn add_target(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddPositionRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    if req.name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Name cannot be empty".to_string(),
            }),
        ));
    }

    let mut targets = state.targets.write().await;

    // Check if already exists
    if targets.iter().any(|t| t.name == req.name) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!("Target '{}' already exists", req.name),
            }),
        ));
    }

    targets.push(Position::new(req.name.clone(), req.elevation, req.x, req.y));

    Ok(Json(SuccessResponse {
        success: true,
        message: format!("Target '{}' added", req.name),
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

// =====================
// Main
// =====================
#[tokio::main]
async fn main() {
    // Determine data path - check multiple locations
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

    let state = Arc::new(AppState {
        ballistics,
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
        .route("/api/ammo-types", get(get_ammo_types))
        // Calculate
        .route("/api/calculate", post(calculate))
        .route("/api/calculate-by-name", post(calculate_by_name))
        // Mortars CRUD
        .route("/api/mortars", get(list_mortars))
        .route("/api/mortars", post(add_mortar))
        .route("/api/mortars", delete(delete_mortar))
        // Targets CRUD
        .route("/api/targets", get(list_targets))
        .route("/api/targets", post(add_target))
        .route("/api/targets", delete(delete_target))
        // Static files
        .nest_service("/", ServeDir::new(web_path))
        .with_state(state);

    let addr = "0.0.0.0:3000";
    println!("Server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

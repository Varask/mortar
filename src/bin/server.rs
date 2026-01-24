use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tower_http::services::ServeDir;

use mortar::{calculate_solution, load_ballistics_from, AmmoKind, BallisticTable, FiringSolution, Position, Ring};

// =====================
// Application state
// =====================
struct AppState {
    ballistics: BTreeMap<(AmmoKind, Ring), BallisticTable>,
}

// =====================
// API types
// =====================
#[derive(Debug, Deserialize)]
struct CalculateRequest {
    mortar: Position,
    target: Position,
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

    let state = Arc::new(AppState { ballistics });

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
        .route("/api/health", get(health_check))
        .route("/api/ammo-types", get(get_ammo_types))
        .route("/api/calculate", post(calculate))
        .nest_service("/", ServeDir::new(web_path))
        .with_state(state);

    let addr = "0.0.0.0:3000";
    println!("Server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

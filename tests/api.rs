use axum::http::StatusCode;
use reqwest::Client;
use std::net::TcpListener;

#[tokio::test]
async fn health_ok() {
    let base = spawn_app().await;
    let client = Client::new();

    let res = client
        .get(format!("{base}/api/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    #[derive(serde::Deserialize)]
    struct HealthResponse {
        status: String,
        version: String,
    }

    let body: HealthResponse = res.json().await.unwrap();
    assert_eq!(body.status, "ok");
    assert!(!body.version.is_empty());
}

#[derive(serde::Serialize)]
struct NewMortar<'a> {
    name: &'a str,
    elevation: f64,
    x: f64,
    y: f64,
    ammo_type: &'a str,
}

#[derive(serde::Serialize)]
struct NewTarget<'a> {
    name: &'a str,
    elevation: f64,
    x: f64,
    y: f64,
    target_type: &'a str,
}

#[derive(serde::Serialize)]
struct CalcRequest<'a> {
    mortar_name: &'a str,
    target_name: &'a str,
}

#[tokio::test]
async fn full_happy_path_returns_firing_solution_json() {
    let base = spawn_app().await;
    let client = Client::new();

    // Add mortar
    let mortar = NewMortar {
        name: "M1",
        elevation: 100.0,
        x: 0.0,
        y: 0.0,
        ammo_type: "HE",
    };

    let res = client
        .post(format!("{base}/api/mortars"))
        .json(&mortar)
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    // Add target
    let target = NewTarget {
        name: "T1",
        elevation: 50.0,
        x: 500.0,
        y: 300.0,
        target_type: "INFANTERIE",
    };

    let res = client
        .post(format!("{base}/api/targets"))
        .json(&target)
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    // Calculate
    let payload = CalcRequest {
        mortar_name: "M1",
        target_name: "T1",
    };

    let res = client
        .post(format!("{base}/api/calculate"))
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert!(res.status().is_success());

    #[derive(serde::Deserialize)]
    struct SelectedSolutionDto {
        ammo_type: String,
    }

    #[derive(serde::Deserialize)]
    struct FiringSolutionDto {
        distance_m: f64,
        azimuth_deg: f64,
        elevation_diff_m: f64,
        signed_elevation_diff_m: f64,
        mortar_ammo: String,
        target_type: String,
        recommended_ammo: String,
        selected_solution: Option<SelectedSolutionDto>,
    }

    let body: FiringSolutionDto = res.json().await.unwrap();
    assert!(body.distance_m > 0.0);
    assert!(body.azimuth_deg >= 0.0 && body.azimuth_deg <= 360.0);
    assert_eq!(body.mortar_ammo, "HE");
    assert_eq!(body.target_type, "INFANTERIE");
    assert_eq!(body.recommended_ammo, "HE");
    assert!(body.selected_solution.is_some());
}

#[tokio::test]
async fn types_endpoint_returns_known_values() {
    let base = spawn_app().await;
    let client = Client::new();

    let res = client
        .get(format!("{base}/api/types"))
        .send()
        .await
        .unwrap();

    assert!(res.status().is_success());

    #[derive(serde::Deserialize)]
    struct TypesResponse {
        ammo_types: Vec<String>,
        target_types: Vec<String>,
    }

    let body: TypesResponse = res.json().await.unwrap();
    assert!(body.ammo_types.contains(&"HE".to_string()));
    assert!(body.ammo_types.contains(&"PRACTICE".to_string()));
    assert!(body.target_types.contains(&"INFANTERIE".to_string()));
    assert!(body.target_types.contains(&"VEHICULE".to_string()));
}

#[tokio::test]
async fn web_assets_are_served() {
    let base = spawn_app().await;
    let client = Client::new();

    // index.html
    let res = client.get(format!("{base}/")).send().await.unwrap();
    assert!(res.status().is_success());
    let body = res.text().await.unwrap();
    assert!(body.contains("<title>Mortar Calculator</title>"));
    assert!(body.contains("Calculateur de Solution de Tir"));

    // style.css
    let res = client.get(format!("{base}/style.css")).send().await.unwrap();
    assert!(res.status().is_success());

    // app.js
    let res = client.get(format!("{base}/app.js")).send().await.unwrap();
    assert!(res.status().is_success());
}

// Helper: start the same router as main, but bound to 127.0.0.1:0
async fn spawn_app() -> String {
    use axum::Router;
    use std::sync::Arc;
    use mortar::{
        load_ballistics_from, load_dispersion_from, AmmoKind, BallisticTable, DispersionTable,
        MortarPosition, Ring, TargetPosition, TargetType,
    };
    use tokio::sync::RwLock;
    use tower_http::services::ServeDir;

    // Construct AppState exactly like main(), but for tests we can point to ./data
    let ballistics = load_ballistics_from("data").unwrap_or_default();
    let dispersions = load_dispersion_from("data").unwrap_or_default();

    struct AppState {
        ballistics: std::collections::BTreeMap<(AmmoKind, Ring), BallisticTable>,
        dispersions: DispersionTable,
        mortars: RwLock<Vec<MortarPosition>>,
        targets: RwLock<Vec<TargetPosition>>,
    }

    let state = Arc::new(AppState {
        ballistics,
        dispersions,
        mortars: RwLock::new(Vec::new()),
        targets: RwLock::new(Vec::new()),
    });

    let web_path = "src/web";

    let app = Router::new()
        .route("/api/health", axum::routing::get(crate::health_check))
        .route("/api/types", axum::routing::get(crate::get_types))
        .route("/api/ammo-types", axum::routing::get(crate::get_ammo_types))
        .route("/api/calculate", axum::routing::post(crate::calculate_by_name))
        .route("/api/mortars", axum::routing::get(crate::list_mortars))
        .route("/api/mortars", axum::routing::post(crate::add_mortar))
        .route("/api/mortars", axum::routing::delete(crate::delete_mortar))
        .route("/api/mortars/ammo", axum::routing::post(crate::update_mortar_ammo))
        .route("/api/targets", axum::routing::get(crate::list_targets))
        .route("/api/targets", axum::routing::post(crate::add_target))
        .route("/api/targets", axum::routing::delete(crate::delete_target))
        .route("/api/targets/type", axum::routing::post(crate::update_target_type))
        .route("/api/targets/correct", axum::routing::post(crate::correct_target))
        .nest_service("/", ServeDir::new(web_path))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind failed");
    let port = listener.local_addr().unwrap().port();
    let addr = format!("http://127.0.0.1:{port}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    addr
}

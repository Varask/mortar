use reqwest::Client;
use serde_json::Value;
use tokio::net::TcpListener;

struct TestApp {
    base_url: String,
    client: Client,
}

fn repo_paths() -> (String, String) {
    // Make paths robust in CI and locally
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data = root.join("data");
    let web = root.join("src").join("web");
    (
        data.to_string_lossy().to_string(),
        web.to_string_lossy().to_string(),
    )
}

async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind failed");
    let port = listener.local_addr().unwrap().port();
    let base_url = format!("http://127.0.0.1:{port}");

    let (data_path, web_path) = repo_paths();
    let app = mortar::server::build_app(&data_path, &web_path);

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server failed");
    });

    TestApp {
        base_url,
        client: Client::new(),
    }
}

#[tokio::test]
async fn health_ok() {
    let app = spawn_app().await;

    let res = app
        .client
        .get(format!("{}/api/health", app.base_url))
        .send()
        .await
        .unwrap();

    assert!(res.status().is_success());

    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(!body["version"].as_str().unwrap_or("").is_empty());
}

#[derive(serde::Serialize)]
struct NewMortar<'a> {
    name: &'a str,
    elevation: f64,
    x: f64,
    y: f64,
}

#[derive(serde::Serialize)]
struct NewTarget<'a> {
    name: &'a str,
    elevation: f64,
    x: f64,
    y: f64,
    target_type: &'a str,
    ammo_type: &'a str,
}

#[derive(serde::Serialize)]
struct CalcRequest<'a> {
    mortar_name: &'a str,
    target_name: &'a str,
}

#[tokio::test]
async fn full_happy_path_returns_firing_solution_json() {
    let app = spawn_app().await;

    // Add mortar
    let res = app
        .client
        .post(format!("{}/api/mortars", app.base_url))
        .json(&NewMortar {
            name: "M1",
            elevation: 100.0,
            x: 0.0,
            y: 0.0,
        })
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    // Add target
    let res = app
        .client
        .post(format!("{}/api/targets", app.base_url))
        .json(&NewTarget {
            name: "T1",
            elevation: 50.0,
            x: 500.0,
            y: 300.0,
            target_type: "INFANTERIE",
            ammo_type: "HE",
        })
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    // Calculate
    let res = app
        .client
        .post(format!("{}/api/calculate", app.base_url))
        .json(&CalcRequest {
            mortar_name: "M1",
            target_name: "T1",
        })
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    // Be tolerant to small schema changes by asserting key presence and types.
    let body: Value = res.json().await.unwrap();

    let distance = body["distance_m"].as_f64().unwrap_or(0.0);
    assert!(distance > 0.0, "distance_m should be > 0, got {distance}");

    let az = body["azimuth_deg"].as_f64().unwrap_or(-1.0);
    assert!(
        (0.0..=360.0).contains(&az),
        "azimuth_deg should be in [0,360], got {az}"
    );

    // These are expected by your current server implementation; if you rename them later,
    // adjust or remove these assertions.
    assert_eq!(body["mortar_ammo"].as_str().unwrap_or(""), "HE");
    assert_eq!(body["target_type"].as_str().unwrap_or(""), "INFANTERIE");
    assert_eq!(body["recommended_ammo"].as_str().unwrap_or(""), "HE");

    assert!(
        body.get("selected_solution").is_some(),
        "expected selected_solution key to exist"
    );
}

#[tokio::test]
async fn web_assets_are_served() {
    let app = spawn_app().await;

    // index
    let res = app
        .client
        .get(format!("{}/", app.base_url))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    let html = res.text().await.unwrap();
    assert!(!html.trim().is_empty());

    // Strong assertions matching your current src/web/index.html
    assert!(
        html.contains("<title>Mortar Calculator</title>"),
        "index.html should contain the expected <title>"
    );
    assert!(
        html.contains("<h1>Calculateur de Solution de Tir</h1>"),
        "index.html should contain the expected <h1>"
    );
    assert!(
        html.contains("<p class=\"subtitle\">Systeme Mortar 60mm</p>"),
        "index.html should contain the expected subtitle"
    );

    // Keep a generic HTML sanity check too
    assert!(
        html.contains("<html") || html.contains("<!DOCTYPE html>"),
        "expected HTML document"
    );

    // static files
    let res = app
        .client
        .get(format!("{}/style.css", app.base_url))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    let res = app
        .client
        .get(format!("{}/app.js", app.base_url))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());
}

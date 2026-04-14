use axum::{
    Json, Router,
    extract::{RawQuery, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{SecondsFormat, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, timeout};
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct AppState {
    client: Arc<Client>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Success,
    Error,
}

#[derive(Serialize)]
struct SuccessResponse {
    status: Status,
    data: DataInfo,
}

#[derive(Serialize)]
struct ErrorResponse {
    status: Status,
    message: String,
}

#[derive(Serialize)]
struct DataInfo {
    name: String,
    gender: String,
    probability: f64,
    sample_size: i64,
    is_confident: bool,
    processed_at: String,
}

#[derive(Deserialize, Serialize)]
struct GenderizeResponse {
    name: String,
    gender: Option<String>,
    probability: Option<f64>,
    count: Option<u32>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ApiResponse {
    Success(SuccessResponse),
    Error(ErrorResponse),
}

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to create HTTP client");

    let state = AppState {
        client: Arc::new(client),
    };

    let app = Router::new()
        .route("/api/classify", get(handler))
        .route("/health", get(health))
        .with_state(state)
        .layer(cors);

    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Server running on http://{addr}/api/classify");
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> impl IntoResponse {
    StatusCode::OK
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(ApiResponse::Error(ErrorResponse {
            status: Status::Error,
            message: message.to_string(),
        })),
    )
        .into_response()
}

async fn handler(
    RawQuery(raw_query): RawQuery,
    State(state): State<AppState>,
) -> Response {
    let raw_query = match raw_query {
        Some(q) => q,
        None => return error_response(StatusCode::BAD_REQUEST, "name parameter is required and cannot be empty"),
    };

    // If `name` is provided as a non-string-ish structure (e.g. `name[]=john`),
    // urlencoded decoding into a `HashMap<String, String>` will fail.
    let params: HashMap<String, String> = match serde_urlencoded::from_str(&raw_query) {
        Ok(p) => p,
        Err(_) => return error_response(StatusCode::UNPROCESSABLE_ENTITY, "name is not a string"),
    };

    let name = params.get("name").map(|s| s.trim()).unwrap_or("");

    // -----------------------------
    // VALIDATION
    // -----------------------------
    if name.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "name parameter is required and cannot be empty",
        );
    }

    // -----------------------------
    // EXTERNAL API CALL
    // -----------------------------
    let response = match timeout(
        Duration::from_secs(3),
        state
            .client
            .get("https://api.genderize.io")
            .query(&[("name", name)])
            .send(),
    )
    .await
    {
        Ok(Ok(res)) => res,
        _ => return error_response(StatusCode::BAD_GATEWAY, "Failed to fetch external API"),
    };
    if !response.status().is_success() {
        return error_response(StatusCode::BAD_GATEWAY, "Failed to call external API");
    }

    let gender_data: GenderizeResponse = match response.json().await {
        Ok(data) => data,
        Err(_) => return error_response(StatusCode::BAD_GATEWAY, "Invalid response from external API"),
    };

    // -----------------------------
    // EDGE CASE
    // -----------------------------
    let gender = match gender_data.gender.as_deref() {
        Some(g) if !g.trim().is_empty() => g.trim(),
        _ => return error_response(StatusCode::UNPROCESSABLE_ENTITY, "No prediction available for the provided name"),
    };

    let sample_size = gender_data.count.unwrap_or(0) as i64;
    if sample_size == 0 {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "No prediction available for the provided name",
        );
    }

    let probability = gender_data.probability.unwrap_or(0.0);

    let is_confident = probability >= 0.7 && sample_size >= 100;

    let data = DataInfo {
        name: gender_data.name,
        gender: gender.to_string(),
        probability,
        sample_size,
        is_confident,
        processed_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
    };

    (
        StatusCode::OK,
        Json(ApiResponse::Success(SuccessResponse {
            status: Status::Success,
            data,
        })),
    )
        .into_response()
}

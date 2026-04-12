use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use tower_http::cors::{CorsLayer, Any};
use reqwest::Client;
use std::sync::Arc;
use tokio::time::{timeout, Duration};

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

#[derive(Deserialize)]
struct NameQuery {
    name: String,
}

#[derive(Deserialize)]
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
        .with_state(state)
        .layer(cors);

    // ✅ Fly-compatible port handling
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    println!("Server running on http://{}/api/classify", addr);

    axum::serve(listener, app).await.unwrap();
}

async fn handler(
    Query(params): Query<NameQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let name = params.name.trim().to_string();

    // -----------------------------
    // VALIDATION
    // -----------------------------
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::Error(ErrorResponse {
                status: Status::Error,
                message: "name parameter is required".to_string(),
            })),
        );
    }

    // -----------------------------
    // EXTERNAL API CALL
    // -----------------------------
    let url = format!("https://api.genderize.io?name={}", name);

    let request_future = state.client.get(&url).send();

    let response = match timeout(Duration::from_secs(5), request_future).await {
        Ok(Ok(res)) => res,
        _ => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ApiResponse::Error(ErrorResponse {
                    status: Status::Error,
                    message: "Failed to fetch external API".to_string(),
                })),
            );
        }
    };

    let gender_data: GenderizeResponse = match response.json().await {
        Ok(data) => data,
        Err(_) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ApiResponse::Error(ErrorResponse {
                    status: Status::Error,
                    message: "Invalid response from external API".to_string(),
                })),
            );
        }
    };

    // -----------------------------
    // EDGE CASE
    // -----------------------------
    let gender = match gender_data.gender {
        Some(g) => g,
        None => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ApiResponse::Error(ErrorResponse {
                    status: Status::Error,
                    message: "No gender prediction available".to_string(),
                })),
            );
        }
    };

    let probability = gender_data.probability.unwrap_or(0.0);
    let sample_size = gender_data.count.unwrap_or(0) as i64;

    let is_confident = probability >= 0.7 && sample_size >= 100;

    let data = DataInfo {
        name: gender_data.name,
        gender,
        probability,
        sample_size,
        is_confident,
        processed_at: Utc::now().to_rfc3339(),
    };

    (
        StatusCode::OK,
        Json(ApiResponse::Success(SuccessResponse {
            status: Status::Success,
            data,
        })),
    )
}

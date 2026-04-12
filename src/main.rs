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
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    let state = AppState {
        client: Arc::new(client),
    };

    let app = Router::new()
        .route("/api/classify", get(handler))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("Server running on http://0.0.0.0:3000/api/classify");
    axum::serve(listener, app).await.unwrap();
}

async fn handler(
    Query(params): Query<NameQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let name = params.name.trim();

    // -----------------------------
    // VALIDATION (400)
    // -----------------------------
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::Error(ErrorResponse {
                status: Status::Error,
                message: "name parameter is required and cannot be empty".to_string(),
            })),
        );
    }

    // -----------------------------
    // CALL EXTERNAL API
    // -----------------------------
    let url = format!("https://api.genderize.io?name={}", name);

    let response = match timeout(Duration::from_secs(3), state.client.get(&url).send()).await {
        Ok(Ok(res)) => res,
        _ => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ApiResponse::Error(ErrorResponse {
                    status: Status::Error,
                    message: "Failed to call external API".to_string(),
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
                    message: "Invalid response format from external API".to_string(),
                })),
            );
        }
    };

    // -----------------------------
    // EDGE CASE
    // -----------------------------
    if gender_data.gender.is_none() || gender_data.count.unwrap_or(0) == 0 {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::Error(ErrorResponse {
                status: Status::Error,
                message: "No prediction available for the provided name".to_string(),
            })),
        );
    }

    // -----------------------------
    // PROCESS DATA
    // -----------------------------
    let probability = gender_data.probability.unwrap_or(0.0);
    let sample_size = gender_data.count.unwrap_or(0) as i64;

    let is_confident = probability >= 0.7 && sample_size >= 100;

    let data = DataInfo {
        name: gender_data.name,
        gender: gender_data.gender.unwrap_or("unknown".to_string()),
        probability,
        sample_size,
        is_confident,
        processed_at: Utc::now().to_rfc3339(),
    };

    // -----------------------------
    // SUCCESS RESPONSE
    // -----------------------------
    (
        StatusCode::OK,
        Json(ApiResponse::Success(SuccessResponse {
            status: Status::Success,
            data,
        })),
    )
}
use axum::{
    Json, Router,
    extract::rejection::QueryRejection,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
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
struct NameQuery {
    #[serde(default)]
    name: Option<String>,
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

async fn handler(
    params: Result<Query<NameQuery>, QueryRejection>,
    State(state): State<AppState>,
) -> Response {
    let params = match params {
        Ok(Query(params)) => params,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::Error(ErrorResponse {
                    status: Status::Error,
                    message: "name parameter is required and cannot be empty".to_string(),
                })),
            )
                .into_response();
        }
    };

    let name = params.name.as_deref().unwrap_or("").trim();

    // -----------------------------
    // VALIDATION
    // -----------------------------
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::Error(ErrorResponse {
                status: Status::Error,
                message: "name parameter is required and cannot be empty".to_string(),
            })),
        )
            .into_response();
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
        _ => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ApiResponse::Error(ErrorResponse {
                    status: Status::Error,
                    message: "Failed to fetch external API".to_string(),
                })),
            )
                .into_response();
        }
    };
    if !response.status().is_success() {
        return (
            StatusCode::BAD_GATEWAY,
            Json(ApiResponse::Error(ErrorResponse {
                status: Status::Error,
                message: "Failed to call external API".to_string(),
            })),
        )
            .into_response();
    }

    let gender_data: GenderizeResponse = match response.json().await {
        Ok(data) => data,
        Err(_) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ApiResponse::Error(ErrorResponse {
                    status: Status::Error,
                    message: "Invalid response from external API".to_string(),
                })),
            )
                .into_response();
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
        )
            .into_response();
    }

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

    (
        StatusCode::OK,
        Json(ApiResponse::Success(SuccessResponse {
            status: Status::Success,
            data,
        })),
    )
        .into_response()
}

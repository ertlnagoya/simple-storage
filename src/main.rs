use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
};

use axum::{
    body::Bytes,
    extract::{Multipart, Query},
    http::{self, StatusCode},
    response::IntoResponse,
    routing::{get, put},
    Router,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    fs::create_dir_all("uploads").unwrap();

    // 404 handler
    let app = Router::new()
        .route("/", get(health_check))
        .route("/upload", put(upload_file))
        .route("/upload/mul", put(upload_mul))
        .route("/upload", get(list_upload))
        .route("/download", get(download))
        .fallback(handler_404);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

// health check
async fn health_check() -> impl IntoResponse {
    tracing::info!("GET /");
    StatusCode::OK
}

// upload file
async fn upload_file(
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> impl IntoResponse {
    tracing::info!("PUT /upload");

    let filename = match query.get("filename") {
        Some(filename) => filename,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let upload_path = format!("uploads/{}", filename);
    let mut file = match File::create(upload_path) {
        Ok(file) => file,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    if file.write_all(&body).is_err() || file.flush().is_err() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    Ok(StatusCode::CREATED)
}

// list uploaded files
async fn list_upload() -> impl IntoResponse {
    tracing::info!("GET /upload");
    let files: Vec<String> = match fs::read_dir("uploads") {
        Ok(files) => files
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect(),
        _ => {
            return axum::Json(Vec::new());
        }
    };
    axum::Json(files)
}

// 404 handler
async fn handler_404() -> impl IntoResponse {
    tracing::info!("404 Not Found");
    StatusCode::NOT_FOUND
}

// download file
// FIXME: like send_from_directory in flask, c.File(filePath)
async fn download(query: Query<HashMap<String, String>>) -> impl IntoResponse {
    tracing::info!("GET /download");
    let filename = match query.get("filename") {
        Some(filename) => filename,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let upload_path = format!("uploads/{}", filename);
    let body = fs::read(upload_path).unwrap();
    // set header
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::CONTENT_DISPOSITION,
        http::HeaderValue::from_str(&format!("attachment; filename={}", filename)).unwrap(),
    );
    Ok((StatusCode::OK, body))
}

async fn upload_mul(mut multipart: Multipart) -> impl IntoResponse {
    tracing::info!("PUT /upload");
    let field = match multipart.next_field().await.unwrap() {
        Some(field) => field,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let name = field.name().unwrap().to_string();
    let file_name = field.file_name().unwrap().to_string();
    let content_type = field.content_type().unwrap().to_string();
    let data = field.bytes().await.unwrap();
    tracing::info!(
        "Length of `{name}` (`{file_name}`: `{content_type}`) is {} bytes",
        data.len()
    );

    let upload_path = format!("uploads/{}", file_name);
    let mut file = match File::create(upload_path) {
        Ok(file) => file,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    if file.write_all(&data).is_err() || file.flush().is_err() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    Ok(StatusCode::CREATED)
}

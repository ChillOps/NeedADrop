use axum::{
    extract::DefaultBodyLimit,
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use sqlx::SqlitePool;
use std::path::PathBuf;
use tokio::fs;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::info;

mod models;
mod handlers;
mod templates;
mod auth;
mod database;

use handlers::*;
use database::*;
use auth::auth_middleware;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub upload_dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    // Load environment variables
    dotenvy::dotenv().ok();
    
    // Initialize database
    let db = init_database().await?;
    
    // Create upload directory
    let upload_dir = PathBuf::from("uploads");
    fs::create_dir_all(&upload_dir).await?;
    
    let state = AppState {
        db,
        upload_dir,
    };
    
    // Build the application router
    let app = Router::new()
        // Public routes
        .route("/", get(index))
        .route("/upload/:token", get(upload_form))
        .route("/upload/:token", post(handle_upload))
        .route("/login", get(login_form))
        .route("/login", post(handle_login))
        
        // Admin routes (require authentication) - nested with middleware
        .nest(
            "/admin",
            Router::new()
                .route("/", get(admin_dashboard))
                .route("/links", get(admin_links))
                .route("/links/create", get(create_link_form))
                .route("/links/create", post(handle_create_link))
                .route("/links/:id/delete", post(delete_link))
                .route("/uploads", get(admin_uploads))
                .route("/uploads/:id/download", get(download_file))
                .route("/uploads/:id/delete", post(delete_upload))
                .route("/change-password", get(change_password_form))
                .route("/change-password", post(handle_change_password))
                .route_layer(middleware::from_fn(auth_middleware))
        )
        .route("/logout", post(logout))
        
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB default
        )
        .with_state(state);
    
    info!("Starting server on http://localhost:3000");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn index() -> impl IntoResponse {
    templates::IndexTemplate.into_response()
}

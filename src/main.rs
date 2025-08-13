//! # NeedADrop - Secure File Upload Application
//!
//! A secure file upload application built with Rust and Axum 0.8.
//! Features quota-based uploads, admin interface, and session-based authentication.
//!
//! ## Architecture Overview
//! - **Web Framework**: Axum 0.8 with Tower 0.5 service layer
//! - **Database**: SQLite with rusqlite 0.37
//! - **Authentication**: Session-based with bcrypt password hashing
//! - **File Storage**: Local filesystem with UUID-based isolation
//! - **Logging**: Structured logging with tracing crate

// Import core web framework dependencies
use axum::{
    extract::DefaultBodyLimit, // For setting request body size limits
    middleware,                // For custom middleware integration
    response::IntoResponse,    // Trait for converting types to HTTP responses
    routing::{get, post},      // HTTP method routing helpers
    Router,                    // Main router type for building the application
};
use std::{path::PathBuf, sync::Arc}; // Standard library types for file paths and thread-safe references
use tokio::fs; // Async filesystem operations
use tower::ServiceBuilder; // Service layer builder for middleware composition
use tower_http::{
    // HTTP-specific middleware from tower-http 0.6
    cors::CorsLayer,    // Cross-Origin Resource Sharing middleware
    services::ServeDir, // Static file serving
    trace::TraceLayer,  // HTTP request/response tracing
};
use tracing::info; // Structured logging macros

// Application modules
mod auth; // Authentication and session management
mod database; // Database operations and initialization
mod handlers; // HTTP request handlers
mod models; // Data models and structures
mod templates; // HTML template rendering

// Import specific items from modules
use auth::auth_middleware; // Authentication middleware for protected routes
use database::*; // Database initialization and operations
use handlers::*; // All HTTP request handlers

/// Application state shared across all handlers
///
/// This struct contains the shared resources that all request handlers need access to:
/// - Database connection pool (wrapped in Arc<Mutex> for thread safety)
/// - Upload directory path for file storage
#[derive(Clone)]
pub struct AppState {
    /// Thread-safe database connection shared across all handlers
    /// Using Arc<Mutex<rusqlite::Connection>> for SQLite connection sharing
    pub db: Arc<std::sync::Mutex<rusqlite::Connection>>,

    /// Base directory where uploaded files are stored
    /// Each upload link gets its own subdirectory using UUID
    pub upload_dir: PathBuf,
}

/// Main application entry point
///
/// Initializes the web server with the following components:
/// 1. Structured logging system with configurable levels
/// 2. Environment variable loading for configuration
/// 3. SQLite database initialization and schema setup
/// 4. Upload directory creation
/// 5. Axum router with public and protected routes
/// 6. Middleware stack for CORS, tracing, and authentication
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging system with environment-based configuration
    // Default level is INFO, can be overridden with RUST_LOG env variable
    init_logging();

    // Load environment variables from .env file (if present)
    // This allows configuration without hardcoding values
    dotenvy::dotenv().ok();

    // Initialize SQLite database connection and create tables if they don't exist
    // This also creates the default admin user if none exists
    let db = init_database()?;

    // Create the upload directory structure
    // Each upload link will get its own UUID-based subdirectory
    let upload_dir = PathBuf::from("uploads");
    fs::create_dir_all(&upload_dir).await?;

    // Create shared application state that will be available to all handlers
    let state = AppState { db, upload_dir };

    // Build the main application router with all routes and middleware
    let app = Router::new()
        // === PUBLIC ROUTES (no authentication required) ===
        // Home page - displays basic application information
        .route("/", get(index))
        // File upload routes for guests with valid tokens
        // GET: Display upload form  POST: Handle file upload
        .route("/upload/:token", get(upload_form))
        .route("/upload/:token", post(handle_upload))
        // Admin authentication routes
        // GET: Display login form  POST: Process login credentials
        .route("/login", get(login_form))
        .route("/login", post(handle_login))
        // === ADMIN ROUTES (authentication required) ===
        // All routes under /admin are protected by auth_middleware
        .nest(
            "/admin",
            Router::new()
                // Admin dashboard with statistics
                .route("/", get(admin_dashboard))
                // Upload link management
                .route("/links", get(admin_links)) // View all upload links
                .route("/links/create", get(create_link_form)) // Create new link form
                .route("/links/create", post(handle_create_link)) // Process new link
                .route("/links/:id/delete", post(delete_link)) // Delete upload link
                // File management
                .route("/uploads", get(admin_uploads)) // View all uploaded files
                .route("/uploads/:id/download", get(download_file)) // Download specific file
                .route("/uploads/:id/delete", post(delete_upload)) // Delete uploaded file
                // Admin account management
                .route("/change-password", get(change_password_form)) // Password change form
                .route("/change-password", post(handle_change_password)) // Process password change
                // Apply authentication middleware to all nested routes
                // This ensures only logged-in admins can access these endpoints
                .route_layer(middleware::from_fn(auth_middleware)),
        )
        // Logout route (available to authenticated users)
        .route("/logout", post(logout))
        // === STATIC FILE SERVING ===
        // Serve CSS, JS, images, and other static assets from the /static directory
        .nest_service("/static", ServeDir::new("static"))
        // === MIDDLEWARE STACK ===
        // Applied in reverse order (last added = first executed)
        .layer(
            ServiceBuilder::new()
                // HTTP request/response tracing for debugging and monitoring
                .layer(TraceLayer::new_for_http())
                // CORS policy - permissive for development (should be restrictive in production)
                .layer(CorsLayer::permissive())
                // Set maximum request body size to 100MB for file uploads
                // This prevents memory exhaustion from extremely large uploads
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024)),
        )
        // Attach the application state to the router
        // This makes the state available to all handlers via the State extractor
        .with_state(state);

    // Log server startup
    info!("Starting server on http://localhost:3000");

    // Create TCP listener and start the server
    // Binds to all interfaces (0.0.0.0) on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Home page handler
///
/// Returns the main index page with application information and links to admin login.
/// This is the only page accessible without any authentication.
async fn index() -> impl IntoResponse {
    templates::IndexTemplate.into_response()
}

/// Initialize the structured logging system
///
/// Sets up tracing with the following features:
/// - Environment-based log level configuration (RUST_LOG)
/// - Structured output with key-value pairs
/// - Thread ID tracking for async debugging
/// - File and line number information
/// - Module target information
///
/// Default log level is INFO, but can be overridden with RUST_LOG environment variable:
/// - `RUST_LOG=debug` for detailed debugging
/// - `RUST_LOG=warn` for warnings and errors only
/// - `RUST_LOG=needadrop=debug,info` for module-specific levels
fn init_logging() {
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    // Parse log level from environment variable with fallback to INFO
    // This allows runtime configuration without recompiling
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("needadrop=info,info"));

    // Build and initialize the subscriber with formatting and filtering
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_target(true) // Include module names in output
                .with_thread_ids(true) // Include thread IDs for async debugging
                .with_file(true) // Include source file names
                .with_line_number(true), // Include line numbers
        )
        .with(env_filter)
        .init();

    info!("Logging system initialized with structured output");
}

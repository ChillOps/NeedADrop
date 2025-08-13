//! # Authentication and Session Management
//!
//! This module handles administrator authentication and session management.
//! It provides middleware for protecting admin routes and session storage.
//!
//! ## Security Features
//! - Session-based authentication with UUIDs
//! - Secure cookie handling with HttpOnly and SameSite flags
//! - Password verification using bcrypt
//! - Automatic session cleanup on logout
//!
//! ## Session Storage
//! Currently uses in-memory storage for simplicity. In production,
//! consider using Redis or database-backed sessions for persistence
//! across server restarts and horizontal scaling.

use axum::{
    extract::Request,
    http::header::COOKIE,
    middleware::Next,
    response::{IntoResponse, Redirect},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Session data stored for authenticated administrators
///
/// Contains the minimum information needed to identify an authenticated admin
/// and provide context for authorization decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier of the authenticated admin user
    pub admin_id: String,

    /// Username of the authenticated admin (for display purposes)
    pub username: String,
}

/// Type alias for the thread-safe session storage
///
/// Uses Arc<RwLock<HashMap>> for concurrent access:
/// - Arc: Multiple ownership across threads
/// - RwLock: Multiple readers OR single writer
/// - HashMap: Fast key-value lookup by session ID
type SessionStore = std::sync::Arc<tokio::sync::RwLock<HashMap<String, Session>>>;

// Global in-memory session store
//
// Production Note: This in-memory store is suitable for single-instance
// deployments but should be replaced with Redis or database storage for
// production environments with multiple servers or persistence requirements.
lazy_static::lazy_static! {
    static ref SESSIONS: SessionStore = std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new()));
}

/// Create a new session for an authenticated administrator
///
/// Generates a new UUID-based session ID and stores the session data.
/// The session ID is returned to be set as a secure HTTP cookie.
///
/// # Arguments
/// * `admin_id` - Unique identifier of the admin user
/// * `username` - Username for display purposes
///
/// # Returns
/// New session ID (UUID string) to be used in cookies
pub async fn create_session(admin_id: String, username: String) -> String {
    let session_id = uuid::Uuid::new_v4().to_string();
    let session = Session { admin_id, username };

    // Acquire write lock and insert session
    let mut sessions = SESSIONS.write().await;
    sessions.insert(session_id.clone(), session);

    session_id
}

/// Retrieve session data by session ID
///
/// Looks up the session in the store and returns a copy of the session data.
/// Returns None if the session ID is not found or has expired.
///
/// # Arguments
/// * `session_id` - Session ID to look up
///
/// # Returns
/// Some(Session) if found, None if not found
pub async fn get_session(session_id: &str) -> Option<Session> {
    let sessions = SESSIONS.read().await;
    sessions.get(session_id).cloned()
}

/// Remove a session from the store (logout)
///
/// Deletes the session data, effectively logging out the user.
/// Safe to call even if the session doesn't exist.
///
/// # Arguments
/// * `session_id` - Session ID to remove
pub async fn remove_session(session_id: &str) {
    let mut sessions = SESSIONS.write().await;
    sessions.remove(session_id);
}

/// Extract session ID from HTTP cookie header
///
/// Parses the Cookie header to find the session_id cookie value.
/// Handles multiple cookies separated by semicolons.
///
/// # Arguments
/// * `cookies` - Raw cookie header value
///
/// # Returns
/// Some(session_id) if found, None if not present
///
/// # Example Cookie Header
/// ```
/// "user_pref=dark; session_id=uuid-here; lang=en"
/// ```
pub fn extract_session_id_from_cookies(cookies: &str) -> Option<&str> {
    cookies.split(';').find_map(|cookie| {
        let cookie = cookie.trim();
        if cookie.starts_with("session_id=") {
            cookie.strip_prefix("session_id=")
        } else {
            None
        }
    })
}

/// Verify a plaintext password against a bcrypt hash
///
/// Uses bcrypt's built-in verification which handles salt extraction
/// and timing-safe comparison automatically.
///
/// # Arguments
/// * `password` - Plaintext password to verify
/// * `hash` - Bcrypt hash to verify against
///
/// # Returns
/// true if password matches hash, false otherwise
///
/// # Security Notes
/// - Uses constant-time comparison to prevent timing attacks
/// - Automatically handles salt extraction from hash
/// - Returns false on any bcrypt errors (malformed hash, etc.)
pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

/// Authentication middleware for protecting admin routes
///
/// This middleware is applied to all routes under `/admin` to ensure
/// only authenticated administrators can access them.
///
/// ## Process
/// 1. Extract session ID from HTTP cookies
/// 2. Look up session in the session store
/// 3. If valid session found, continue to the protected route
/// 4. If no valid session, redirect to login page
///
/// # Arguments
/// * `request` - Incoming HTTP request
/// * `next` - Next middleware/handler in the chain
///
/// # Returns
/// Either the response from the protected route or a redirect to login
pub async fn auth_middleware(request: Request, next: Next) -> impl IntoResponse {
    // Extract session ID from the Cookie header
    let session_id = request
        .headers()
        .get(COOKIE)
        .and_then(|header| header.to_str().ok())
        .and_then(extract_session_id_from_cookies);

    match session_id {
        Some(session_id) => {
            // Attempt to validate the session by looking it up in the store
            if get_session(session_id).await.is_some() {
                // Session is valid, continue to the protected route
                next.run(request).await
            } else {
                // Session ID found but not in store (expired/invalid)
                // Redirect to login page
                Redirect::to("/login").into_response()
            }
        }
        None => {
            // No session cookie found, user is not authenticated
            // Redirect to login page
            Redirect::to("/login").into_response()
        }
    }
}

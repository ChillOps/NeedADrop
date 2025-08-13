// Session management for admin authentication
use axum::{
    extract::Request,
    http::header::COOKIE,
    middleware::Next,
    response::{IntoResponse, Redirect},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub admin_id: String,
    pub username: String,
}

// Simple in-memory session store (in production, use Redis or database)
type SessionStore = std::sync::Arc<tokio::sync::RwLock<HashMap<String, Session>>>;

lazy_static::lazy_static! {
    static ref SESSIONS: SessionStore = std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new()));
}

pub async fn create_session(admin_id: String, username: String) -> String {
    let session_id = uuid::Uuid::new_v4().to_string();
    let session = Session { admin_id, username };
    
    let mut sessions = SESSIONS.write().await;
    sessions.insert(session_id.clone(), session);
    
    session_id
}

pub async fn get_session(session_id: &str) -> Option<Session> {
    let sessions = SESSIONS.read().await;
    sessions.get(session_id).cloned()
}

pub async fn remove_session(session_id: &str) {
    let mut sessions = SESSIONS.write().await;
    sessions.remove(session_id);
}

pub fn extract_session_id_from_cookies(cookies: &str) -> Option<&str> {
    cookies
        .split(';')
        .find_map(|cookie| {
            let cookie = cookie.trim();
            if cookie.starts_with("session_id=") {
                cookie.strip_prefix("session_id=")
            } else {
                None
            }
        })
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

pub async fn auth_middleware(request: Request, next: Next) -> impl IntoResponse {
    // Extract session ID from cookies
    let session_id = request
        .headers()
        .get(COOKIE)
        .and_then(|header| header.to_str().ok())
        .and_then(extract_session_id_from_cookies);

    match session_id {
        Some(session_id) => {
            // Validate session
            if get_session(session_id).await.is_some() {
                // Session is valid, continue to the next handler
                next.run(request).await
            } else {
                // Invalid session, redirect to login
                Redirect::to("/login").into_response()
            }
        }
        None => {
            // No session cookie, redirect to login
            Redirect::to("/login").into_response()
        }
    }
}

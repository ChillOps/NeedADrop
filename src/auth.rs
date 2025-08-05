// Session management for admin authentication
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



pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

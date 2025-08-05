use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UploadLink {
    pub id: String,
    pub token: String,
    pub name: String,
    pub max_file_size: i64, // total quota in bytes
    pub remaining_quota: i64, // remaining quota in bytes
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FileUpload {
    pub id: String,
    pub link_id: String,
    pub original_filename: String,
    pub stored_filename: String,
    pub file_size: i64,
    pub mime_type: String,
    pub uploaded_at: DateTime<Utc>,
    pub guest_folder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Admin {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLinkForm {
    pub name: String,
    pub max_file_size_mb: f64,
    pub expires_in_hours: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordForm {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

impl UploadLink {
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }
    
    pub fn is_valid(&self) -> bool {
        self.is_active && !self.is_expired() && self.remaining_quota > 0
    }
    
    pub fn can_accept_file(&self, file_size: i64) -> bool {
        self.is_valid() && self.remaining_quota >= file_size
    }
}

impl FileUpload {
    pub fn file_path(&self, upload_dir: &std::path::Path) -> std::path::PathBuf {
        upload_dir.join(&self.guest_folder).join(&self.stored_filename)
    }
}

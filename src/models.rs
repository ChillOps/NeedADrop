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
    #[serde(deserialize_with = "deserialize_optional_int")]
    pub expires_in_hours: Option<i32>,
}

fn deserialize_optional_int<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    if s.trim().is_empty() {
        Ok(None)
    } else {
        s.trim().parse::<i32>().map(Some).map_err(serde::de::Error::custom)
    }
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
    
    pub fn formatted_max_size(&self) -> String {
        format_file_size(self.max_file_size)
    }
    
    pub fn formatted_remaining_quota(&self) -> String {
        format_file_size(self.remaining_quota)
    }
}

impl FileUpload {
    pub fn file_path(&self, upload_dir: &std::path::Path) -> std::path::PathBuf {
        upload_dir.join(&self.guest_folder).join(&self.stored_filename)
    }
    
    pub fn formatted_size(&self) -> String {
        format_file_size(self.file_size)
    }
}

/// Format file size in bytes to human readable format
pub fn format_file_size(size_bytes: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: f64 = 1024.0;
    
    if size_bytes == 0 {
        return "0 B".to_string();
    }
    
    let size = size_bytes as f64;
    let unit_index = (size.log10() / THRESHOLD.log10()).floor() as usize;
    let unit_index = unit_index.min(UNITS.len() - 1);
    
    if unit_index == 0 {
        format!("{} B", size_bytes)
    } else {
        let value = size / THRESHOLD.powi(unit_index as i32);
        format!("{:.1} {}", value, UNITS[unit_index])
    }
}

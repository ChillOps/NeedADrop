//! # Data Models and Structures
//!
//! This module contains all the data models used throughout the application.
//! All models implement Serialize/Deserialize for JSON API compatibility
//! and database operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Upload Link Model
///
/// Represents a unique upload link created by administrators.
/// Each link has a quota system and optional expiration.
///
/// ## Security Features
/// - UUID-based tokens prevent enumeration attacks
/// - Quota system prevents storage abuse
/// - Time-based expiration for access control
/// - Active/inactive states for link management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadLink {
    /// Unique identifier for the upload link (UUID)
    pub id: String,

    /// Public token used in URLs (UUID) - safe to expose to guests
    pub token: String,

    /// Human-readable name for the link (set by admin)
    pub name: String,

    /// Total quota in bytes - maximum total file size allowed
    pub max_file_size: i64,

    /// Remaining quota in bytes - decreases with each upload
    pub remaining_quota: i64,

    /// Optional expiration time - link becomes invalid after this time
    pub expires_at: Option<DateTime<Utc>>,

    /// When the link was created
    pub created_at: DateTime<Utc>,

    /// Whether the link is active (admin can deactivate without deleting)
    pub is_active: bool,
}

/// File Upload Model
///
/// Represents an uploaded file associated with an upload link.
/// Files are stored with UUID-based names for security.
///
/// ## Storage Strategy
/// - Original filename preserved for downloads
/// - UUID-based stored filename prevents conflicts
/// - Guest folder isolation (one folder per upload session)
/// - MIME type detection and storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUpload {
    /// Unique identifier for this file upload (UUID)
    pub id: String,

    /// Foreign key to the upload link that this file belongs to
    pub link_id: String,

    /// Original filename as uploaded by the user
    pub original_filename: String,

    /// UUID-based filename used for storage (prevents conflicts)
    pub stored_filename: String,

    /// File size in bytes
    pub file_size: i64,

    /// MIME type detected during upload
    pub mime_type: String,

    /// When the file was uploaded
    pub uploaded_at: DateTime<Utc>,

    /// UUID-based folder where this file is stored (guest isolation)
    pub guest_folder: String,
}

/// Administrator User Model
///
/// Represents an administrator account with password authentication.
/// Passwords are stored as bcrypt hashes for security.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Admin {
    /// Unique identifier for the admin user (UUID)
    pub id: String,

    /// Username for login (must be unique)
    pub username: String,

    /// Bcrypt hash of the password (never store plaintext passwords)
    pub password_hash: String,

    /// When the admin account was created
    pub created_at: DateTime<Utc>,
}

// === Form Models for HTML Forms ===
// These models handle form data from the web interface

/// Form data for creating new upload links
///
/// Submitted by administrators when creating new upload links.
/// File size is collected in MB for user convenience and converted to bytes.
#[derive(Debug, Deserialize)]
pub struct CreateLinkForm {
    /// Human-readable name for the upload link
    pub name: String,

    /// Maximum file size quota in megabytes (converted to bytes in handler)
    pub max_file_size_mb: f64,

    /// Optional expiration time in hours from now
    /// Uses custom deserializer to handle empty form fields
    #[serde(deserialize_with = "deserialize_optional_int")]
    pub expires_in_hours: Option<i32>,
}

/// Custom deserializer for optional integer fields from HTML forms
///
/// HTML forms submit empty fields as empty strings, but we want None for optional integers.
/// This function converts empty strings to None and parses non-empty strings to Some(i32).
fn deserialize_optional_int<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    if s.trim().is_empty() {
        Ok(None)
    } else {
        s.trim()
            .parse::<i32>()
            .map(Some)
            .map_err(serde::de::Error::custom)
    }
}

/// Form data for admin login
///
/// Simple form with username and password fields for administrator authentication.
#[derive(Debug, Deserialize)]
pub struct LoginForm {
    /// Administrator username
    pub username: String,

    /// Password (will be verified against bcrypt hash)
    pub password: String,
}

/// Form data for changing admin password
///
/// Requires current password for verification and new password with confirmation.
#[derive(Debug, Deserialize)]
pub struct ChangePasswordForm {
    /// Current password for verification
    pub current_password: String,

    /// New password to set
    pub new_password: String,

    /// Confirmation of new password (must match new_password)
    pub confirm_password: String,
}

// === Business Logic Implementation ===
// Methods that implement business rules and validation

impl UploadLink {
    /// Check if the upload link has expired based on its expiration time
    ///
    /// Returns true if the link has an expiration time and it has passed.
    /// Links without expiration times never expire.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false // No expiration time means never expires
        }
    }

    /// Check if the upload link is valid and can accept uploads
    ///
    /// A link is valid if:
    /// - It is marked as active by admin
    /// - It has not expired
    /// - It has remaining quota (> 0 bytes)
    pub fn is_valid(&self) -> bool {
        self.is_active && !self.is_expired() && self.remaining_quota > 0
    }

    /// Check if the upload link can accept a specific file size
    ///
    /// Returns true if the link is valid and has enough remaining quota
    /// to accommodate the specified file size.
    pub fn can_accept_file(&self, file_size: i64) -> bool {
        self.is_valid() && self.remaining_quota >= file_size
    }

    /// Format the maximum file size in a human-readable format
    ///
    /// Converts bytes to appropriate units (B, KB, MB, GB) for display.
    pub fn formatted_max_size(&self) -> String {
        format_file_size(self.max_file_size)
    }
}

impl FileUpload {
    /// Construct the full filesystem path for this uploaded file
    ///
    /// Combines the base upload directory with the guest folder and stored filename
    /// to create the complete path where the file is stored on disk.
    ///
    /// # Arguments
    /// * `upload_dir` - Base directory where all uploads are stored
    ///
    /// # Returns
    /// Complete path to the file: `upload_dir/guest_folder/stored_filename`
    pub fn file_path(&self, upload_dir: &std::path::Path) -> std::path::PathBuf {
        upload_dir
            .join(&self.guest_folder)
            .join(&self.stored_filename)
    }

    /// Format the file size in a human-readable format
    ///
    /// Converts bytes to appropriate units (B, KB, MB, GB) for display.
    pub fn formatted_size(&self) -> String {
        format_file_size(self.file_size)
    }
}

// === Utility Functions ===

/// Format file size in bytes to human-readable format
///
/// Converts raw byte counts to appropriate units with proper formatting:
/// - 0-1023 bytes: "X B"
/// - 1024+ bytes: "X.Y KB/MB/GB/TB" (1 decimal place)
///
/// # Arguments
/// * `size_bytes` - File size in bytes
///
/// # Returns
/// Formatted string like "1.5 MB", "512 B", "2.3 GB"
///
/// # Examples
/// ```
/// assert_eq!(format_file_size(0), "0 B");
/// assert_eq!(format_file_size(512), "512 B");
/// assert_eq!(format_file_size(1536), "1.5 KB");
/// assert_eq!(format_file_size(1048576), "1.0 MB");
/// ```
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
        // For bytes, show exact count without decimals
        format!("{} B", size_bytes)
    } else {
        // For larger units, show one decimal place
        let value = size / THRESHOLD.powi(unit_index as i32);
        format!("{:.1} {}", value, UNITS[unit_index])
    }
}

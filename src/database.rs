use crate::models::*;
use chrono::Utc;
use sqlx::SqlitePool;
use std::path::Path;
use tokio::fs;
use tracing::{debug, error, info};
use uuid::Uuid;

pub async fn init_database() -> Result<SqlitePool, sqlx::Error> {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:needadrop.db".to_string());

    info!(database_url = %database_url, "Initializing database");

    // Extract the file path from the database URL
    let db_path = database_url
        .strip_prefix("sqlite:")
        .unwrap_or(&database_url);

    // Create the database file if it doesn't exist
    if !Path::new(db_path).exists() {
        info!(db_path = %db_path, "Database file doesn't exist, creating new one");

        // Create parent directories if they don't exist
        if let Some(parent) = Path::new(db_path).parent() {
            debug!(parent_dir = %parent.display(), "Creating database parent directories");
            fs::create_dir_all(parent).await.map_err(|e| {
                error!(parent_dir = %parent.display(), error = %e, "Failed to create database directory");
                sqlx::Error::Io(std::io::Error::other(format!(
                    "Failed to create database directory: {}",
                    e
                )))
            })?;
        }

        // Create empty database file
        fs::File::create(db_path).await.map_err(|e| {
            error!(db_path = %db_path, error = %e, "Failed to create database file");
            sqlx::Error::Io(std::io::Error::other(format!(
                "Failed to create database file: {}",
                e
            )))
        })?;
    } else {
        debug!(db_path = %db_path, "Database file already exists");
    }

    debug!("Connecting to database");
    let pool = SqlitePool::connect(&database_url).await.map_err(|e| {
        error!(database_url = %database_url, error = %e, "Failed to connect to database");
        e
    })?;

    // Run migrations
    info!("Running database migrations");
    create_tables(&pool).await?;

    // Create default admin user if none exists
    info!("Checking for default admin user");
    create_default_admin(&pool).await?;

    info!("Database initialization completed successfully");
    Ok(pool)
}

async fn create_tables(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Create admins table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS admins (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create upload_links table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS upload_links (
            id TEXT PRIMARY KEY,
            token TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            max_file_size INTEGER NOT NULL,
            remaining_quota INTEGER NOT NULL DEFAULT 0,
            expires_at TEXT,
            created_at TEXT NOT NULL,
            is_active BOOLEAN NOT NULL DEFAULT 1
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Add remaining_quota column if it doesn't exist (migration)
    let _ = sqlx::query("ALTER TABLE upload_links ADD COLUMN remaining_quota INTEGER DEFAULT 0")
        .execute(pool)
        .await;

    // Update existing links to have remaining_quota = max_file_size if remaining_quota is 0
    sqlx::query(
        "UPDATE upload_links SET remaining_quota = max_file_size WHERE remaining_quota = 0",
    )
    .execute(pool)
    .await?;

    // Create file_uploads table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS file_uploads (
            id TEXT PRIMARY KEY,
            link_id TEXT NOT NULL,
            original_filename TEXT NOT NULL,
            stored_filename TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            mime_type TEXT NOT NULL,
            uploaded_at TEXT NOT NULL,
            guest_folder TEXT NOT NULL,
            FOREIGN KEY (link_id) REFERENCES upload_links (id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_default_admin(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admins")
        .fetch_one(pool)
        .await?;

    if count == 0 {
        let admin_id = Uuid::new_v4().to_string();
        let password_hash = bcrypt::hash("admin123", bcrypt::DEFAULT_COST)
            .map_err(|e| sqlx::Error::Protocol(format!("Password hashing failed: {}", e)))?;

        sqlx::query(
            "INSERT INTO admins (id, username, password_hash, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&admin_id)
        .bind("admin")
        .bind(&password_hash)
        .bind(Utc::now().to_rfc3339())
        .execute(pool)
        .await?;

        tracing::info!("Default admin user created: username=admin, password=admin123");
    }

    Ok(())
}

pub async fn get_admin_by_username(
    pool: &SqlitePool,
    username: &str,
) -> Result<Option<Admin>, sqlx::Error> {
    let admin = sqlx::query_as::<_, Admin>(
        "SELECT id, username, password_hash, created_at FROM admins WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    Ok(admin)
}

pub async fn create_upload_link(
    pool: &SqlitePool,
    name: String,
    max_file_size: i64,
    expires_at: Option<chrono::DateTime<Utc>>,
) -> Result<UploadLink, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let token = Uuid::new_v4().to_string();
    let created_at = Utc::now();
    let remaining_quota = max_file_size; // Initially, remaining quota equals max size

    let link = UploadLink {
        id: id.clone(),
        token: token.clone(),
        name: name.clone(),
        max_file_size,
        remaining_quota,
        expires_at,
        created_at,
        is_active: true,
    };

    sqlx::query(
        "INSERT INTO upload_links (id, token, name, max_file_size, remaining_quota, expires_at, created_at, is_active) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&link.id)
    .bind(&link.token)
    .bind(&link.name)
    .bind(link.max_file_size)
    .bind(link.remaining_quota)
    .bind(link.expires_at.as_ref().map(|dt| dt.to_rfc3339()))
    .bind(link.created_at.to_rfc3339())
    .bind(link.is_active)
    .execute(pool)
    .await?;

    Ok(link)
}
pub async fn get_upload_link_by_token(
    pool: &SqlitePool,
    token: &str,
) -> Result<Option<UploadLink>, sqlx::Error> {
    let link = sqlx::query_as::<_, UploadLink>(
        "SELECT id, token, name, max_file_size, remaining_quota, expires_at, created_at, is_active FROM upload_links WHERE token = ?"
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(link)
}

pub async fn get_upload_link_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<UploadLink>, sqlx::Error> {
    let link = sqlx::query_as::<_, UploadLink>(
        "SELECT id, token, name, max_file_size, remaining_quota, expires_at, created_at, is_active FROM upload_links WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(link)
}

pub async fn get_all_upload_links(pool: &SqlitePool) -> Result<Vec<UploadLink>, sqlx::Error> {
    let links = sqlx::query_as::<_, UploadLink>(
        "SELECT id, token, name, max_file_size, remaining_quota, expires_at, created_at, is_active FROM upload_links ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(links)
}

pub async fn delete_upload_link(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM upload_links WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn create_file_upload(
    pool: &SqlitePool,
    link_id: String,
    original_filename: String,
    stored_filename: String,
    file_size: i64,
    mime_type: String,
    guest_folder: String,
) -> Result<FileUpload, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let uploaded_at = Utc::now();

    let upload = FileUpload {
        id: id.clone(),
        link_id,
        original_filename,
        stored_filename,
        file_size,
        mime_type,
        uploaded_at,
        guest_folder,
    };

    sqlx::query(
        "INSERT INTO file_uploads (id, link_id, original_filename, stored_filename, file_size, mime_type, uploaded_at, guest_folder) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&upload.id)
    .bind(&upload.link_id)
    .bind(&upload.original_filename)
    .bind(&upload.stored_filename)
    .bind(upload.file_size)
    .bind(&upload.mime_type)
    .bind(upload.uploaded_at.to_rfc3339())
    .bind(&upload.guest_folder)
    .execute(pool)
    .await?;

    Ok(upload)
}

pub async fn get_all_file_uploads(pool: &SqlitePool) -> Result<Vec<FileUpload>, sqlx::Error> {
    let uploads = sqlx::query_as::<_, FileUpload>(
        "SELECT id, link_id, original_filename, stored_filename, file_size, mime_type, uploaded_at, guest_folder FROM file_uploads ORDER BY uploaded_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(uploads)
}

pub async fn get_file_uploads_by_link_id(
    pool: &SqlitePool,
    link_id: &str,
) -> Result<Vec<FileUpload>, sqlx::Error> {
    let uploads = sqlx::query_as::<_, FileUpload>(
        "SELECT id, link_id, original_filename, stored_filename, file_size, mime_type, uploaded_at, guest_folder FROM file_uploads WHERE link_id = ? ORDER BY uploaded_at DESC"
    )
    .bind(link_id)
    .fetch_all(pool)
    .await?;

    Ok(uploads)
}

pub async fn get_file_upload_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<FileUpload>, sqlx::Error> {
    let upload = sqlx::query_as::<_, FileUpload>(
        "SELECT id, link_id, original_filename, stored_filename, file_size, mime_type, uploaded_at, guest_folder FROM file_uploads WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(upload)
}

pub async fn update_admin_password(
    pool: &SqlitePool,
    username: &str,
    new_password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE admins SET password_hash = ? WHERE username = ?")
        .bind(new_password_hash)
        .bind(username)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_remaining_quota(
    pool: &SqlitePool,
    link_id: &str,
    uploaded_size: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE upload_links SET remaining_quota = remaining_quota - ? WHERE id = ?")
        .bind(uploaded_size)
        .bind(link_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_file_upload(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM file_uploads WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

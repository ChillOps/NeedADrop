use crate::models::*;
use chrono::Utc;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tracing::{debug, info};
use uuid::Uuid;

pub fn init_database() -> Result<Arc<Mutex<Connection>>, Box<dyn std::error::Error>> {
    let database_path = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "needadrop.db".to_string())
        .replace("sqlite:", "");

    info!(database_path = %database_path, "Initializing database");

    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(&database_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    debug!("Connecting to database");
    let conn = Connection::open(&database_path)?;

    info!("Running database migrations");
    create_tables(&conn)?;

    info!("Checking for default admin user");
    create_default_admin(&conn)?;

    info!("Database initialization completed successfully");
    Ok(Arc::new(Mutex::new(conn)))
}

fn create_tables(conn: &Connection) -> SqliteResult<()> {
    // Create admins table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS admins (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL
        )
        "#,
        [],
    )?;

    // Create upload_links table
    conn.execute(
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
        [],
    )?;

    // Create file_uploads table
    conn.execute(
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
            FOREIGN KEY (link_id) REFERENCES upload_links (id) ON DELETE CASCADE
        )
        "#,
        [],
    )?;

    // Try to add the remaining_quota column if it doesn't exist (migration)
    let _ = conn.execute(
        "ALTER TABLE upload_links ADD COLUMN remaining_quota INTEGER DEFAULT 0",
        [],
    );

    // Update existing links to set remaining_quota to max_file_size if it's 0
    conn.execute(
        "UPDATE upload_links SET remaining_quota = max_file_size WHERE remaining_quota = 0",
        [],
    )?;

    Ok(())
}

fn create_default_admin(conn: &Connection) -> SqliteResult<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM admins", [], |row| row.get(0))?;

    if count == 0 {
        let admin_id = Uuid::new_v4().to_string();
        let password_hash = bcrypt::hash("admin123", bcrypt::DEFAULT_COST)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        conn.execute(
            "INSERT INTO admins (id, username, password_hash, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![admin_id, "admin", password_hash, Utc::now().to_rfc3339()],
        )?;

        info!("Created default admin user: admin/admin123");
    }

    Ok(())
}

// Database query functions
pub fn get_admin_by_username(
    db: &Arc<Mutex<Connection>>,
    username: &str,
) -> Result<Option<Admin>, Box<dyn std::error::Error>> {
    let conn = db.lock().unwrap();

    let mut stmt = conn
        .prepare("SELECT id, username, password_hash, created_at FROM admins WHERE username = ?")?;

    let admin_result = stmt.query_row([username], |row| {
        Ok(Admin {
            id: row.get(0)?,
            username: row.get(1)?,
            password_hash: row.get(2)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                .unwrap()
                .with_timezone(&Utc),
        })
    });

    match admin_result {
        Ok(admin) => Ok(Some(admin)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(Box::new(e)),
    }
}

pub fn create_upload_link(
    db: &Arc<Mutex<Connection>>,
    name: &str,
    max_file_size: i64,
    expires_at: Option<chrono::DateTime<Utc>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let conn = db.lock().unwrap();

    let link_id = Uuid::new_v4().to_string();
    let token = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO upload_links (id, token, name, max_file_size, remaining_quota, expires_at, created_at, is_active) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            &link_id,
            &token,
            name,
            max_file_size,
            max_file_size, // remaining_quota starts as max_file_size
            expires_at.map(|dt| dt.to_rfc3339()),
            Utc::now().to_rfc3339(),
            true,
        ],
    )?;

    Ok(token)
}

pub fn get_upload_link_by_token(
    db: &Arc<Mutex<Connection>>,
    token: &str,
) -> Result<Option<UploadLink>, Box<dyn std::error::Error>> {
    let conn = db.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT id, token, name, max_file_size, remaining_quota, expires_at, created_at, is_active FROM upload_links WHERE token = ?"
    )?;

    let link_result = stmt.query_row([token], |row| {
        let expires_at_str: Option<String> = row.get(5)?;
        let expires_at = expires_at_str.map(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .unwrap()
                .with_timezone(&Utc)
        });

        Ok(UploadLink {
            id: row.get(0)?,
            token: row.get(1)?,
            name: row.get(2)?,
            max_file_size: row.get(3)?,
            remaining_quota: row.get(4)?,
            expires_at,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .unwrap()
                .with_timezone(&Utc),
            is_active: row.get(7)?,
        })
    });

    match link_result {
        Ok(link) => Ok(Some(link)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(Box::new(e)),
    }
}

pub fn get_upload_link_by_id(
    db: &Arc<Mutex<Connection>>,
    id: &str,
) -> Result<Option<UploadLink>, Box<dyn std::error::Error>> {
    let conn = db.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT id, token, name, max_file_size, remaining_quota, expires_at, created_at, is_active FROM upload_links WHERE id = ?"
    )?;

    let link_result = stmt.query_row([id], |row| {
        let expires_at_str: Option<String> = row.get(5)?;
        let expires_at = expires_at_str.map(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .unwrap()
                .with_timezone(&Utc)
        });

        Ok(UploadLink {
            id: row.get(0)?,
            token: row.get(1)?,
            name: row.get(2)?,
            max_file_size: row.get(3)?,
            remaining_quota: row.get(4)?,
            expires_at,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .unwrap()
                .with_timezone(&Utc),
            is_active: row.get(7)?,
        })
    });

    match link_result {
        Ok(link) => Ok(Some(link)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(Box::new(e)),
    }
}

pub fn get_all_upload_links(
    db: &Arc<Mutex<Connection>>,
) -> Result<Vec<UploadLink>, Box<dyn std::error::Error>> {
    let conn = db.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT id, token, name, max_file_size, remaining_quota, expires_at, created_at, is_active FROM upload_links ORDER BY created_at DESC"
    )?;

    let link_iter = stmt.query_map([], |row| {
        let expires_at_str: Option<String> = row.get(5)?;
        let expires_at = expires_at_str.map(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .unwrap()
                .with_timezone(&Utc)
        });

        Ok(UploadLink {
            id: row.get(0)?,
            token: row.get(1)?,
            name: row.get(2)?,
            max_file_size: row.get(3)?,
            remaining_quota: row.get(4)?,
            expires_at,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .unwrap()
                .with_timezone(&Utc),
            is_active: row.get(7)?,
        })
    })?;

    let mut links = Vec::new();
    for link in link_iter {
        links.push(link?);
    }

    Ok(links)
}

pub fn delete_upload_link(
    db: &Arc<Mutex<Connection>>,
    id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = db.lock().unwrap();

    conn.execute("DELETE FROM upload_links WHERE id = ?", [id])?;

    Ok(())
}

pub fn create_file_upload(
    db: &Arc<Mutex<Connection>>,
    link_id: &str,
    original_filename: &str,
    stored_filename: &str,
    file_size: i64,
    mime_type: &str,
    guest_folder: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let conn = db.lock().unwrap();

    let id = Uuid::new_v4().to_string();
    let uploaded_at = Utc::now();

    conn.execute(
        "INSERT INTO file_uploads (id, link_id, original_filename, stored_filename, file_size, mime_type, uploaded_at, guest_folder) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            &id,
            link_id,
            original_filename,
            stored_filename,
            file_size,
            mime_type,
            uploaded_at.to_rfc3339(),
            guest_folder,
        ],
    )?;

    Ok(id)
}

pub fn get_all_file_uploads(
    db: &Arc<Mutex<Connection>>,
) -> Result<Vec<FileUpload>, Box<dyn std::error::Error>> {
    let conn = db.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT id, link_id, original_filename, stored_filename, file_size, mime_type, uploaded_at, guest_folder FROM file_uploads ORDER BY uploaded_at DESC"
    )?;

    let upload_iter = stmt.query_map([], |row| {
        Ok(FileUpload {
            id: row.get(0)?,
            link_id: row.get(1)?,
            original_filename: row.get(2)?,
            stored_filename: row.get(3)?,
            file_size: row.get(4)?,
            mime_type: row.get(5)?,
            uploaded_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .unwrap()
                .with_timezone(&Utc),
            guest_folder: row.get(7)?,
        })
    })?;

    let mut uploads = Vec::new();
    for upload in upload_iter {
        uploads.push(upload?);
    }

    Ok(uploads)
}

pub fn get_file_uploads_by_link_id(
    db: &Arc<Mutex<Connection>>,
    link_id: &str,
) -> Result<Vec<FileUpload>, Box<dyn std::error::Error>> {
    let conn = db.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT id, link_id, original_filename, stored_filename, file_size, mime_type, uploaded_at, guest_folder FROM file_uploads WHERE link_id = ? ORDER BY uploaded_at DESC"
    )?;

    let upload_iter = stmt.query_map([link_id], |row| {
        Ok(FileUpload {
            id: row.get(0)?,
            link_id: row.get(1)?,
            original_filename: row.get(2)?,
            stored_filename: row.get(3)?,
            file_size: row.get(4)?,
            mime_type: row.get(5)?,
            uploaded_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .unwrap()
                .with_timezone(&Utc),
            guest_folder: row.get(7)?,
        })
    })?;

    let mut uploads = Vec::new();
    for upload in upload_iter {
        uploads.push(upload?);
    }

    Ok(uploads)
}

pub fn get_file_upload_by_id(
    db: &Arc<Mutex<Connection>>,
    id: &str,
) -> Result<Option<FileUpload>, Box<dyn std::error::Error>> {
    let conn = db.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT id, link_id, original_filename, stored_filename, file_size, mime_type, uploaded_at, guest_folder FROM file_uploads WHERE id = ?"
    )?;

    let upload_result = stmt.query_row([id], |row| {
        Ok(FileUpload {
            id: row.get(0)?,
            link_id: row.get(1)?,
            original_filename: row.get(2)?,
            stored_filename: row.get(3)?,
            file_size: row.get(4)?,
            mime_type: row.get(5)?,
            uploaded_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .unwrap()
                .with_timezone(&Utc),
            guest_folder: row.get(7)?,
        })
    });

    match upload_result {
        Ok(upload) => Ok(Some(upload)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(Box::new(e)),
    }
}

pub fn update_admin_password(
    db: &Arc<Mutex<Connection>>,
    username: &str,
    new_password_hash: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = db.lock().unwrap();

    conn.execute(
        "UPDATE admins SET password_hash = ? WHERE username = ?",
        params![new_password_hash, username],
    )?;

    Ok(())
}

pub fn update_remaining_quota(
    db: &Arc<Mutex<Connection>>,
    link_id: &str,
    uploaded_size: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = db.lock().unwrap();

    conn.execute(
        "UPDATE upload_links SET remaining_quota = remaining_quota - ? WHERE id = ?",
        params![uploaded_size, link_id],
    )?;

    Ok(())
}

pub fn delete_file_upload(
    db: &Arc<Mutex<Connection>>,
    id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = db.lock().unwrap();

    conn.execute("DELETE FROM file_uploads WHERE id = ?", [id])?;

    Ok(())
}

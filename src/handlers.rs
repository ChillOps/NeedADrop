use axum::{
    extract::{rejection::FormRejection, Multipart, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    Form,
};
use chrono::{Duration, Utc};
use tokio::fs;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{auth::*, database::*, models::*, templates::*, AppState};

async fn get_session_from_headers(headers: &HeaderMap) -> Option<Session> {
    let session_id = headers
        .get(header::COOKIE)
        .and_then(|header| header.to_str().ok())
        .and_then(extract_session_id_from_cookies)?;

    get_session(session_id).await
}

pub async fn upload_form(
    Path(token): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    debug!(token = %token, "Accessing upload form");

    match get_upload_link_by_token(&state.db, &token).await {
        Ok(Some(link)) => {
            if link.is_valid() {
                debug!(link_id = %link.id, link_name = %link.name, "Valid upload link accessed");
                UploadTemplate {
                    link,
                    error: None,
                    success: None,
                }
                .into_response()
            } else {
                warn!(token = %token, "Expired or inactive upload link accessed");
                (StatusCode::GONE, "Upload link has expired or is inactive").into_response()
            }
        }
        Ok(None) => {
            warn!(token = %token, "Upload link not found");
            (StatusCode::NOT_FOUND, "Upload link not found").into_response()
        }
        Err(e) => {
            error!(token = %token, error = %e, "Database error while fetching upload link");
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

pub async fn handle_upload(
    Path(token): Path<String>,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    info!(token = %token, "File upload initiated");

    // Get upload link
    let link = match get_upload_link_by_token(&state.db, &token).await {
        Ok(Some(link)) if link.is_valid() => {
            debug!(
                link_id = %link.id,
                link_name = %link.name,
                remaining_quota = link.remaining_quota,
                "Valid upload link found"
            );
            link
        }
        Ok(Some(_)) => {
            warn!(token = %token, "Upload attempted with expired or inactive link");
            return UploadTemplate {
                link: UploadLink {
                    id: String::new(),
                    token: token.clone(),
                    name: "Expired Link".to_string(),
                    max_file_size: 0,
                    remaining_quota: 0,
                    expires_at: None,
                    created_at: Utc::now(),
                    is_active: false,
                },
                error: Some("Upload link has expired or is inactive".to_string()),
                success: None,
            }
            .into_response();
        }
        Ok(None) => {
            warn!(token = %token, "Upload attempted with non-existent link");
            return (StatusCode::NOT_FOUND, "Upload link not found").into_response();
        }
        Err(e) => {
            error!(token = %token, error = %e, "Database error while fetching upload link");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    // Process uploaded file
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let filename = field.file_name().unwrap_or("unnamed_file").to_string();

            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();

            debug!(
                filename = %filename,
                content_type = %content_type,
                link_id = %link.id,
                "Processing uploaded file"
            );

            let data = match field.bytes().await {
                Ok(data) => {
                    info!(
                        filename = %filename,
                        file_size_mb = data.len() as f64 / 1024.0 / 1024.0,
                        link_id = %link.id,
                        "File data read successfully"
                    );
                    data
                }
                Err(e) => {
                    error!(
                        filename = %filename,
                        link_id = %link.id,
                        error = %e,
                        "Failed to read uploaded file"
                    );
                    return UploadTemplate {
                        link: link.clone(),
                        error: Some("Failed to read uploaded file".to_string()),
                        success: None,
                    }
                    .into_response();
                }
            };

            // Check file size against remaining quota
            if !link.can_accept_file(data.len() as i64) {
                warn!(
                    filename = %filename,
                    file_size_mb = data.len() as f64 / 1024.0 / 1024.0,
                    remaining_quota_mb = link.remaining_quota as f64 / 1024.0 / 1024.0,
                    link_id = %link.id,
                    "File size exceeds remaining quota"
                );
                return UploadTemplate {
                    link: link.clone(),
                    error: Some(format!(
                        "File size ({:.1} MB) exceeds remaining quota ({:.1} MB). Total quota: {:.1} MB",
                        data.len() as f64 / 1024.0 / 1024.0,
                        link.remaining_quota as f64 / 1024.0 / 1024.0,
                        link.max_file_size as f64 / 1024.0 / 1024.0
                    )),
                    success: None,
                }
                .into_response();
            }

            // Create guest directory
            let guest_folder = Uuid::new_v4().to_string();
            let guest_dir = state.upload_dir.join(&guest_folder);

            debug!(
                guest_folder = %guest_folder,
                guest_dir = %guest_dir.display(),
                "Creating upload directory"
            );

            if (fs::create_dir_all(&guest_dir).await).is_err() {
                error!(
                    guest_dir = %guest_dir.display(),
                    "Failed to create upload directory"
                );
                return UploadTemplate {
                    link: link.clone(),
                    error: Some("Failed to create upload directory".to_string()),
                    success: None,
                }
                .into_response();
            }

            // Generate unique filename
            let extension = std::path::Path::new(&filename)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");

            let stored_filename = if extension.is_empty() {
                Uuid::new_v4().to_string()
            } else {
                format!("{}.{}", Uuid::new_v4(), extension)
            };

            let file_path = guest_dir.join(&stored_filename);

            debug!(
                original_filename = %filename,
                stored_filename = %stored_filename,
                file_path = %file_path.display(),
                "Generated unique filename"
            );

            // Write file
            match fs::write(&file_path, &data).await {
                Ok(_) => {
                    debug!(
                        file_path = %file_path.display(),
                        file_size = data.len(),
                        "File written to disk successfully"
                    );

                    // Save to database
                    match create_file_upload(
                        &state.db,
                        link.id.clone(),
                        filename.clone(),
                        stored_filename.clone(),
                        data.len() as i64,
                        content_type,
                        guest_folder.clone(),
                    )
                    .await
                    {
                        Ok(_) => {
                            info!(
                                original_filename = %filename,
                                stored_filename = %stored_filename,
                                file_size_mb = data.len() as f64 / 1024.0 / 1024.0,
                                link_id = %link.id,
                                guest_folder = %guest_folder,
                                "File upload completed successfully"
                            );

                            // Update remaining quota
                            if (update_remaining_quota(&state.db, &link.id, data.len() as i64)
                                .await)
                                .is_err()
                            {
                                // Even if quota update fails, the file was uploaded successfully
                                error!(
                                    link_id = %link.id,
                                    "Failed to update remaining quota for link"
                                );
                            }

                            return UploadTemplate {
                                link: link.clone(),
                                error: None,
                                success: Some("File uploaded successfully!".to_string()),
                            }
                            .into_response();
                        }
                        Err(e) => {
                            error!(
                                original_filename = %filename,
                                stored_filename = %stored_filename,
                                link_id = %link.id,
                                error = %e,
                                "Failed to save upload information to database"
                            );

                            // Clean up file on database error
                            let _ = fs::remove_file(&file_path).await;
                            let _ = fs::remove_dir(&guest_dir).await;

                            return UploadTemplate {
                                link: link.clone(),
                                error: Some("Failed to save upload information".to_string()),
                                success: None,
                            }
                            .into_response();
                        }
                    }
                }
                Err(e) => {
                    error!(
                        file_path = %file_path.display(),
                        error = %e,
                        "Failed to write file to disk"
                    );

                    return UploadTemplate {
                        link: link.clone(),
                        error: Some("Failed to save uploaded file".to_string()),
                        success: None,
                    }
                    .into_response();
                }
            }
        }
    }

    UploadTemplate {
        link,
        error: Some("No file was uploaded".to_string()),
        success: None,
    }
    .into_response()
}

pub async fn login_form() -> impl IntoResponse {
    LoginTemplate { error: None }
}

pub async fn handle_login(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    info!(username = %form.username, "Login attempt");

    match get_admin_by_username(&state.db, &form.username).await {
        Ok(Some(admin)) => {
            debug!(admin_id = %admin.id, username = %admin.username, "Found admin user");

            if verify_password(&form.password, &admin.password_hash) {
                info!(admin_id = %admin.id, username = %admin.username, "Password verification successful");
                let session_id = create_session(admin.id, admin.username).await;

                let redirect = Redirect::to("/admin");
                let mut response = redirect.into_response();

                // Set session cookie
                let cookie = format!(
                    "session_id={}; Path=/; HttpOnly; SameSite=Strict",
                    session_id
                );
                response
                    .headers_mut()
                    .insert(header::SET_COOKIE, cookie.parse().unwrap());

                return response;
            } else {
                warn!(username = %form.username, "Password verification failed");
            }
        }
        Ok(None) => {
            warn!(username = %form.username, "Admin user not found");
        }
        Err(e) => {
            error!(username = %form.username, error = %e, "Database error during login");
        }
    }

    LoginTemplate {
        error: Some("Invalid username or password".to_string()),
    }
    .into_response()
}

pub async fn admin_dashboard(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let session = match get_session_from_headers(&headers).await {
        Some(session) => session,
        None => return Redirect::to("/login").into_response(),
    };

    // Get stats for dashboard
    let active_links_count = match get_all_upload_links(&state.db).await {
        Ok(links) => links.iter().filter(|link| link.is_valid()).count(),
        Err(_) => 0,
    };

    let total_uploads_count = match get_all_file_uploads(&state.db).await {
        Ok(uploads) => uploads.len(),
        Err(_) => 0,
    };

    AdminDashboardTemplate {
        username: session.username,
        active_links: active_links_count,
        total_uploads: total_uploads_count,
    }
    .into_response()
}

pub async fn admin_links(headers: HeaderMap, State(state): State<AppState>) -> impl IntoResponse {
    let session = match get_session_from_headers(&headers).await {
        Some(session) => session,
        None => return Redirect::to("/login").into_response(),
    };

    match get_all_upload_links(&state.db).await {
        Ok(links) => AdminLinksTemplate {
            links,
            username: session.username,
            error: None,
        }
        .into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    }
}

pub async fn create_link_form(headers: HeaderMap) -> impl IntoResponse {
    let session = match get_session_from_headers(&headers).await {
        Some(session) => session,
        None => return Redirect::to("/login").into_response(),
    };

    CreateLinkTemplate {
        error: None,
        username: session.username,
    }
    .into_response()
}

pub async fn handle_create_link(
    headers: HeaderMap,
    State(state): State<AppState>,
    form_result: Result<Form<CreateLinkForm>, FormRejection>,
) -> impl IntoResponse {
    let session = match get_session_from_headers(&headers).await {
        Some(session) => session,
        None => return Redirect::to("/login").into_response(),
    };

    // Handle form parsing errors
    let form = match form_result {
        Ok(Form(form)) => form,
        Err(_) => {
            return CreateLinkTemplate {
                error: Some(
                    "Invalid form data. Please check that the expiration time is a valid number."
                        .to_string(),
                ),
                username: session.username,
            }
            .into_response();
        }
    };

    let max_file_size = (form.max_file_size_mb * 1024.0 * 1024.0) as i64;

    // Handle empty expiration field
    let expires_at = if let Some(hours) = form.expires_in_hours {
        if hours > 0 {
            Some(Utc::now() + Duration::hours(hours as i64))
        } else {
            None
        }
    } else {
        None
    };

    match create_upload_link(&state.db, form.name, max_file_size, expires_at).await {
        Ok(_) => Redirect::to("/admin/links").into_response(),
        Err(_) => CreateLinkTemplate {
            error: Some("Failed to create upload link".to_string()),
            username: session.username,
        }
        .into_response(),
    }
}

pub async fn delete_link(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let session = match get_session_from_headers(&headers).await {
        Some(session) => session,
        None => return Redirect::to("/login").into_response(),
    };

    // Check if there are any uploads associated with this link
    match get_file_uploads_by_link_id(&state.db, &id).await {
        Ok(uploads) => {
            if !uploads.is_empty() {
                // There are uploads associated with this link, show error
                let links = get_all_upload_links(&state.db).await.unwrap_or_default();
                return AdminLinksTemplate {
                    links,
                    username: session.username,
                    error: Some("Cannot delete link: it still has uploaded files. Please delete the files first.".to_string()),
                }
                .into_response();
            }
        }
        Err(_) => {
            // Database error checking uploads
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    }

    // No uploads associated, safe to delete
    match delete_upload_link(&state.db, &id).await {
        Ok(_) => Redirect::to("/admin/links").into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete link").into_response(),
    }
}

pub async fn admin_uploads(headers: HeaderMap, State(state): State<AppState>) -> impl IntoResponse {
    let session = match get_session_from_headers(&headers).await {
        Some(session) => session,
        None => return Redirect::to("/login").into_response(),
    };

    match get_all_file_uploads(&state.db).await {
        Ok(uploads) => {
            // Group uploads by link_id
            let mut grouped_uploads: std::collections::HashMap<
                String,
                (UploadLink, Vec<FileUpload>),
            > = std::collections::HashMap::new();

            for upload in uploads {
                if let Ok(Some(link)) = get_upload_link_by_id(&state.db, &upload.link_id).await {
                    grouped_uploads
                        .entry(upload.link_id.clone())
                        .or_insert_with(|| (link, Vec::new()))
                        .1
                        .push(upload);
                } else {
                    // If link is not found, create placeholder
                    let placeholder_link = UploadLink {
                        id: upload.link_id.clone(),
                        token: "unknown".to_string(),
                        name: "Deleted Link".to_string(),
                        max_file_size: 0,
                        remaining_quota: 0,
                        expires_at: None,
                        created_at: Utc::now(),
                        is_active: false,
                    };
                    grouped_uploads
                        .entry(upload.link_id.clone())
                        .or_insert_with(|| (placeholder_link, Vec::new()))
                        .1
                        .push(upload);
                }
            }

            // Convert to sorted vector for template
            let mut grouped_vec: Vec<(UploadLink, Vec<FileUpload>)> =
                grouped_uploads.into_values().collect();
            // Sort by link creation date (newest first)
            grouped_vec.sort_by(|a, b| b.0.created_at.cmp(&a.0.created_at));

            // Sort files within each group by upload date (newest first)
            for (_, uploads) in &mut grouped_vec {
                uploads.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));
            }

            AdminUploadsTemplate {
                grouped_uploads: grouped_vec,
                username: session.username,
            }
            .into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    }
}

pub async fn download_file(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match get_file_upload_by_id(&state.db, &id).await {
        Ok(Some(upload)) => {
            let file_path = upload.file_path(&state.upload_dir);

            match fs::read(&file_path).await {
                Ok(data) => {
                    let headers = [
                        (header::CONTENT_TYPE, upload.mime_type.as_str()),
                        (
                            header::CONTENT_DISPOSITION,
                            &format!("attachment; filename=\"{}\"", upload.original_filename),
                        ),
                    ];

                    (headers, data).into_response()
                }
                Err(_) => (StatusCode::NOT_FOUND, "File not found on disk").into_response(),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Upload not found").into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    }
}

pub async fn delete_upload(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match get_file_upload_by_id(&state.db, &id).await {
        Ok(Some(upload)) => {
            // Delete file from disk
            let file_path = upload.file_path(&state.upload_dir);
            if (fs::remove_file(&file_path).await).is_err() {
                // File might already be deleted, continue with database deletion
            }

            // Delete from database
            match delete_file_upload(&state.db, &id).await {
                Ok(_) => Redirect::to("/admin/uploads").into_response(),
                Err(_) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete upload").into_response()
                }
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Upload not found").into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    }
}

pub async fn change_password_form(headers: HeaderMap) -> impl IntoResponse {
    let session = match get_session_from_headers(&headers).await {
        Some(session) => session,
        None => return Redirect::to("/login").into_response(),
    };

    ChangePasswordTemplate {
        error: None,
        success: None,
        username: session.username,
    }
    .into_response()
}

pub async fn handle_change_password(
    headers: HeaderMap,
    State(state): State<AppState>,
    Form(form): Form<ChangePasswordForm>,
) -> impl IntoResponse {
    let session = match get_session_from_headers(&headers).await {
        Some(session) => session,
        None => return Redirect::to("/login").into_response(),
    };

    // Validate that new passwords match
    if form.new_password != form.confirm_password {
        return ChangePasswordTemplate {
            error: Some("New passwords do not match".to_string()),
            success: None,
            username: session.username,
        }
        .into_response();
    }

    // Validate password length
    if form.new_password.len() < 6 {
        return ChangePasswordTemplate {
            error: Some("Password must be at least 6 characters long".to_string()),
            success: None,
            username: session.username.clone(),
        }
        .into_response();
    }

    // Get current admin user (using session username)
    match get_admin_by_username(&state.db, &session.username).await {
        Ok(Some(admin)) => {
            // Verify current password
            if !verify_password(&form.current_password, &admin.password_hash) {
                return ChangePasswordTemplate {
                    error: Some("Current password is incorrect".to_string()),
                    success: None,
                    username: session.username,
                }
                .into_response();
            }

            // Hash new password
            let new_hash = match bcrypt::hash(&form.new_password, bcrypt::DEFAULT_COST) {
                Ok(hash) => hash,
                Err(_) => {
                    return ChangePasswordTemplate {
                        error: Some("Failed to hash new password".to_string()),
                        success: None,
                        username: session.username,
                    }
                    .into_response();
                }
            };

            // Update password in database
            match update_admin_password(&state.db, &session.username, &new_hash).await {
                Ok(_) => ChangePasswordTemplate {
                    error: None,
                    success: Some("Password changed successfully!".to_string()),
                    username: session.username,
                }
                .into_response(),
                Err(_) => ChangePasswordTemplate {
                    error: Some("Failed to update password in database".to_string()),
                    success: None,
                    username: session.username,
                }
                .into_response(),
            }
        }
        Ok(None) => ChangePasswordTemplate {
            error: Some("Admin user not found".to_string()),
            success: None,
            username: session.username,
        }
        .into_response(),
        Err(_) => ChangePasswordTemplate {
            error: Some("Database error".to_string()),
            success: None,
            username: session.username,
        }
        .into_response(),
    }
}

pub async fn logout(headers: HeaderMap) -> impl IntoResponse {
    // Extract session ID from cookie header and remove it from server-side store
    if let Some(session_id) = headers
        .get(header::COOKIE)
        .and_then(|header| header.to_str().ok())
        .and_then(extract_session_id_from_cookies)
    {
        remove_session(session_id).await;
    }

    let redirect = Redirect::to("/");
    let mut response = redirect.into_response();

    // Clear session cookie
    let cookie = "session_id=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0";
    response
        .headers_mut()
        .insert(header::SET_COOKIE, cookie.parse().unwrap());

    response
}

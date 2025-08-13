use crate::models::*;
use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate;

impl IntoResponse for IndexTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "upload.html")]
pub struct UploadTemplate {
    pub link: UploadLink,
    pub error: Option<String>,
    pub success: Option<String>,
}

impl IntoResponse for UploadTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
}

impl IntoResponse for LoginTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
pub struct AdminDashboardTemplate {
    pub username: String,
    pub active_links: usize,
    pub total_uploads: usize,
}

impl IntoResponse for AdminDashboardTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "admin/links.html")]
pub struct AdminLinksTemplate {
    pub links: Vec<UploadLink>,
    pub username: String,
    pub error: Option<String>,
}

impl IntoResponse for AdminLinksTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "admin/create_link.html")]
pub struct CreateLinkTemplate {
    pub error: Option<String>,
    pub username: String,
}

impl IntoResponse for CreateLinkTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "admin/uploads.html")]
pub struct AdminUploadsTemplate {
    pub grouped_uploads: Vec<(UploadLink, Vec<FileUpload>)>,
    pub username: String,
}

impl IntoResponse for AdminUploadsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response(),
        }
    }
}

impl AdminUploadsTemplate {
    pub fn total_size(&self) -> i64 {
        self.grouped_uploads
            .iter()
            .flat_map(|(_, uploads)| uploads)
            .map(|upload| upload.file_size)
            .sum()
    }

    pub fn formatted_total_size(&self) -> String {
        crate::models::format_file_size(self.total_size())
    }
}

#[derive(Template)]
#[template(path = "admin/change_password.html")]
pub struct ChangePasswordTemplate {
    pub error: Option<String>,
    pub success: Option<String>,
    pub username: String,
}

impl IntoResponse for ChangePasswordTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response(),
        }
    }
}

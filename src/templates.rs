use askama::Template;
use crate::models::*;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate;

#[derive(Template)]
#[template(path = "upload.html")]
pub struct UploadTemplate {
    pub link: UploadLink,
    pub error: Option<String>,
    pub success: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
pub struct AdminDashboardTemplate {
    pub username: String,
    pub active_links: usize,
    pub total_uploads: usize,
}

#[derive(Template)]
#[template(path = "admin/links.html")]
pub struct AdminLinksTemplate {
    pub links: Vec<UploadLink>,
    pub username: String,
}

#[derive(Template)]
#[template(path = "admin/create_link.html")]
pub struct CreateLinkTemplate {
    pub error: Option<String>,
    pub username: String,
}

#[derive(Template)]
#[template(path = "admin/uploads.html")]
pub struct AdminUploadsTemplate {
    pub grouped_uploads: Vec<(UploadLink, Vec<FileUpload>)>,
    pub username: String,
}

#[derive(Template)]
#[template(path = "admin/change_password.html")]
pub struct ChangePasswordTemplate {
    pub error: Option<String>,
    pub success: Option<String>,
    pub username: String,
}

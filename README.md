# NeedADrop

A secure file upload application built with Rust and Axum framework. Features quota-based uploads, admin interface, and session-based authentication.

## Features

- **Secure File Uploads**: Upload files with configurable size limits and quota management
- **Admin Interface**: Manage uploaded files and monitor system usage
- **Quota System**: Per-link quota management to control storage usage
- **Authentication**: Secure session-based authentication with bcrypt password hashing
- **Password Management**: Admin can change passwords through the web interface
- **File Organization**: Files are grouped by upload links with isolated storage
- **Real-time Statistics**: Dashboard shows actual file counts and storage metrics

## Security Features

- No default credentials exposed
- Bcrypt password hashing
- Session-based authentication
- Isolated file storage per upload link
- Input validation and sanitization
- SQL injection protection with SQLx
- CORS protection
- Dependency vulnerability monitoring

## Quick Start with Docker

### Prerequisites
- Docker and Docker Compose installed

### Using Docker Compose (Recommended)

1. Clone the repository:
```bash
git clone <repository-url>
cd NeedADrop
```

2. Start the application:
```bash
docker-compose up -d
```

3. Access the application at `http://localhost:3000`

### Manual Docker Build

1. Build the image:
```bash
docker build -t needadrop .
```

2. Run the container:
```bash
docker run -d \
  --name needadrop \
  -p 3000:3000 \
  -v needadrop_data:/app/data \
  -v needadrop_uploads:/app/uploads \
  needadrop
```

## Development Setup

### Prerequisites
- Rust 1.70+ (latest stable recommended)
- SQLite 3

### Local Development

1. Clone the repository:
```bash
git clone <repository-url>
cd NeedADrop
```

2. Install dependencies:
```bash
cargo build
```

3. Run the application:
```bash
cargo run
```

4. Access the application at `http://localhost:3000`

## Configuration

The application can be configured using environment variables:

- `DATABASE_URL`: SQLite database path (default: `sqlite://needadrop.db`)
- `UPLOAD_DIR`: Directory for file uploads (default: `./uploads`)
- `PORT`: Server port (default: `3000`)
- `RUST_LOG`: Logging level (default: `info`)

## Security Audit

Dependencies are regularly audited for vulnerabilities:

```bash
# Install cargo-audit
cargo install cargo-audit

# Run security audit
cargo audit
```

## Docker Security

The Docker setup includes several security best practices:

- **Multi-stage build**: Minimizes final image size and attack surface
- **Non-root user**: Application runs as unprivileged user
- **Alpine Linux**: Minimal base image with fewer vulnerabilities
- **Read-only filesystem**: Container filesystem is read-only except for specific directories
- **Dropped capabilities**: Minimal Linux capabilities
- **Health checks**: Container health monitoring
- **Tini init system**: Proper signal handling and zombie process reaping

## API Endpoints

- `GET /` - Main upload interface
- `POST /upload` - Upload files
- `GET /admin` - Admin login page
- `POST /admin/login` - Admin authentication
- `GET /admin/dashboard` - Admin dashboard
- `GET /admin/uploads` - View uploaded files
- `POST /admin/delete/<file_id>` - Delete files
- `GET /admin/change-password` - Change password form
- `POST /admin/change-password` - Update password
- `POST /admin/logout` - Admin logout
- `GET /files/<link_id>/<filename>` - Download files

## Database Schema

The application uses SQLite with the following main tables:

- `upload_links`: Stores upload link information and quotas
- `uploaded_files`: Stores file metadata and paths
- `admin_users`: Stores admin credentials (hashed)

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test`
5. Run security audit: `cargo audit`
6. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

- 🔒 **Secure Upload Links**: Administrators create unique, secure upload links with tokens
- ⏰ **Time-Limited Links**: Links can expire after a specified time period
- 📏 **Quota-Based Size Limits**: Set total upload quota per link that decreases with each upload
- 👤 **Guest Isolation**: Each guest upload is stored in a separate, isolated folder
- 🛡️ **Admin-Only Access**: Only administrators can view, download, and manage uploaded files
- 🗄️ **SQLite Database**: Lightweight database for storing metadata
- 🎨 **Clean Web Interface**: Modern, responsive web interface for both guests and admins
- 📊 **Grouped File Management**: Files are organized and displayed by upload link

## Quick Start

### Prerequisites

- Rust (latest stable version)
- SQLite3

### Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd NeedADrop
```

2. Build and run the application:
```bash
cargo run
```

3. Open your browser and navigate to `http://localhost:3000`

### Default Admin Credentials

- **Username**: `admin`
- **Password**: `admin123`

⚠️ **Important**: Change the default password immediately after first login in a production environment.

## Usage

### For Administrators

1. **Login**: Go to `/login` and use the admin credentials
2. **Create Upload Links**: 
   - Navigate to "Manage Upload Links"
   - Click "Create New Link"
   - Set a descriptive name, total upload quota (not per-file limit), and optional expiration time
   - Share the generated upload URL with guests
3. **Manage Uploads**: View all uploaded files grouped by upload link, download them, or delete unwanted uploads
4. **Change Password**: Update admin credentials through the account settings

### For Guests

1. **Access Upload Form**: Use the upload link provided by the administrator
2. **Upload Files**: Select and upload files within the remaining quota limits
3. **Multiple Uploads**: Continue uploading until the quota is exhausted
4. **Confirmation**: Receive confirmation when each upload is successful

## Configuration

### Environment Variables

Create a `.env` file in the project root:

```env
DATABASE_URL=sqlite:needadrop.db
```

### File Storage

- Uploaded files are stored in the `uploads/` directory
- Each guest upload gets its own isolated folder
- Folder structure: `uploads/{guest-folder-uuid}/{stored-filename}`

## API Endpoints

### Public Routes
- `GET /` - Home page
- `GET /upload/{token}` - Upload form for guests
- `POST /upload/{token}` - Handle file upload
- `GET /login` - Admin login form
- `POST /login` - Handle admin login

### Admin Routes (Requires Authentication)
- `GET /admin` - Admin dashboard
- `GET /admin/links` - View all upload links
- `GET /admin/links/create` - Create new upload link form
- `POST /admin/links/create` - Handle link creation
- `POST /admin/links/{id}/delete` - Delete upload link
- `GET /admin/uploads` - View all uploads
- `GET /admin/uploads/{id}/download` - Download file
- `POST /admin/uploads/{id}/delete` - Delete upload
- `GET /admin/change-password` - Change admin password form
- `POST /admin/change-password` - Handle password change
- `POST /logout` - Admin logout

## Security Features

- **Token-based Access**: Upload links use UUID tokens for security
- **Time-based Expiration**: Links can automatically expire
- **Quota Validation**: Server-side quota checking and enforcement
- **Path Isolation**: Guest uploads are isolated in separate directories
- **Admin Authentication**: Bcrypt-hashed passwords with session management
- **Password Management**: Admins can change their passwords securely

## Development

### Project Structure

```
src/
├── main.rs          # Main application entry point
├── models.rs        # Data models and structures
├── database.rs      # Database operations and migrations
├── handlers.rs      # HTTP request handlers
├── auth.rs          # Authentication and session management
└── templates.rs     # Template definitions

templates/           # Askama HTML templates
├── index.html
├── login.html
├── upload.html
└── admin/
    ├── dashboard.html
    ├── links.html
    ├── create_link.html
    └── uploads.html

static/              # Static assets (CSS, JS, images)
uploads/             # File upload storage (created at runtime)
```

### Dependencies

- **axum**: Web framework
- **tokio**: Async runtime
- **sqlx**: Database toolkit
- **askama**: Template engine
- **bcrypt**: Password hashing
- **uuid**: UUID generation
- **chrono**: Date/time handling

### Building for Production

```bash
cargo build --release
```

The binary will be available at `target/release/needadrop`.

## License

This project is open source. Please check the license file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## Support

For issues, questions, or contributions, please open an issue on the repository.
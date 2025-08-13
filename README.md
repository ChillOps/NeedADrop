# NeedADrop

A secure file upload applica## 🔧 Configuration

Environment variables:
- `DATABASE_URL`: SQLite database path (default: `sqlite://needladrop.db`)
- `UPLOAD_DIR`: Directory for uploads (default: `./uploads`)
- `PORT`: Server port (default: `3000`)
- `RUST_LOG`: Logging level (default: `info`)

### 📋 Logging Configuration

NeedADrop uses structured logging with configurable levels:

```bash
# Basic logging levels
RUST_LOG=info                    # Standard production logging
RUST_LOG=debug                   # Detailed debugging information
RUST_LOG=warn                    # Warnings and errors only
RUST_LOG=error                   # Errors only

# Module-specific logging
RUST_LOG=needladrop=debug,info   # Debug for app, info for dependencies
RUST_LOG=needladrop::handlers=debug,needladrop::database=info,warn

# Examples
cargo run                                    # Default INFO level
RUST_LOG=debug cargo run                     # Full debug output
RUST_LOG=needladrop=warn,warn cargo run      # Minimal logging
```

**Log Features:**
- 🎯 **Structured Data**: Key-value pairs for easy parsing
- 📍 **Source Location**: File names and line numbers
- 🧵 **Thread Information**: Multi-threaded request tracking
- 🕐 **Timestamps**: Precise timing for debugging
- 🔍 **Contextual Info**: User IDs, file names, link IDs, etc.

See [LOGGING.md](LOGGING.md) for detailed documentation.ilt with Rust and Axum framework. Features quota-based uploads, admin interface, and session-based authentication.

## ✨ Features

- **🔒 Secure Upload Links**: Administrators create unique, time-limited upload links with tokens
- **📏 Quota-Based Limits**: Set total upload quota per link that decreases with each upload
- **👤 Guest Isolation**: Each upload link stores files in a separate, isolated folder
- **🛡️ Admin-Only Access**: Only administrators can view, download, and manage uploaded files
- **🎨 Modern Interface**: Clean, responsive web interface with glassmorphism design
- **📊 Real-time Statistics**: Dashboard shows actual file counts and storage metrics
- **🔐 Security First**: Bcrypt password hashing, session authentication, SQL injection protection

## 🚀 Quick Start

### Using Docker Compose (Recommended)

1. Clone and start:
```bash
git clone <repository-url>
cd NeedADrop
docker-compose up -d
```

2. Access at `http://localhost:3000`
3. Login with: `admin` / `admin123` (change immediately!)

### Local Development

1. Prerequisites: Rust 1.70+, SQLite 3

2. Run locally:
```bash
git clone <repository-url>
cd NeedADrop
cargo run
```

3. Access at `http://localhost:3000`

## 🔧 Configuration

Environment variables:
- `DATABASE_URL`: SQLite database path (default: `sqlite://needadrop.db`)
- `UPLOAD_DIR`: Directory for uploads (default: `./uploads`)
- `PORT`: Server port (default: `3000`)
- `RUST_LOG`: Logging level (default: `info`)

## 🏗️ CI/CD & GitHub Actions

Complete automation pipeline included:

- **✅ CI Pipeline**: Formatting, linting, tests, security checks
- **🐋 Docker Registry**: Multi-arch builds pushed to GitHub Container Registry
- **🔐 Private Registry**: Support for private Docker registries
- **📦 Release Management**: Automated releases with cross-platform binaries
- **🛡️ Branch Protection**: Comprehensive PR validation

See [GitHub Actions Setup Guide](docs/github-actions-setup.md) for detailed configuration.

## 📁 Usage

### For Administrators

1. **Login**: Navigate to `/admin`
2. **Create Upload Links**: Set name, quota, and optional expiration
3. **Share Links**: Distribute upload URLs to guests
4. **Manage Files**: View, download, or delete uploads by link
5. **Change Password**: Update credentials in admin settings

### For Guests

1. **Access Upload Form**: Use the link provided by admin
2. **Upload Files**: Drag & drop or browse files within quota
3. **Visual Feedback**: Real-time quota usage and file type icons
4. **Multiple Uploads**: Continue until quota is exhausted

## 🛡️ Security Features

- **Token-based Access**: UUID tokens for upload links
- **Time-based Expiration**: Automatic link expiration
- **Quota Validation**: Server-side enforcement
- **Path Isolation**: Separate directories per upload link
- **Authentication**: Bcrypt-hashed passwords with sessions
- **Dependency Auditing**: Regular vulnerability scanning with `cargo audit`

## 🏗️ Project Structure

```
src/
├── main.rs          # Application entry point
├── models.rs        # Data models and structures
├── database.rs      # Database operations
├── handlers.rs      # HTTP request handlers
└── auth.rs          # Authentication & sessions

templates/           # Askama HTML templates
├── upload.html      # Modern file upload interface
├── login.html       # Admin login form
└── admin/           # Admin panel templates

.github/workflows/   # CI/CD automation
├── ci.yml           # Basic CI pipeline
├── docker.yml       # Docker registry builds
├── private-registry.yml # Private registry support
├── release.yml      # Automated releases
└── branch-protection.yml # PR validation
```

## 🛠️ Development

### Local Setup
```bash
# Build and run
cargo run

# Run tests
cargo test

# Security audit
cargo audit

# Format code
cargo fmt
```

### Contributing

1. **Feature branches** from `develop`
2. **Conventional Commits**: `feat:`, `fix:`, `docs:`, etc.
3. **PR Requirements**: Pass all checks, code review
4. **Automated Releases**: Using [release-please](https://github.com/googleapis/release-please)

## 🐋 Docker

### Production Deployment
```bash
# Using Docker Compose
docker-compose up -d

# Manual deployment
docker run -d \
  --name needadrop \
  -p 3000:3000 \
  -v needadrop_data:/app/data \
  -v needadrop_uploads:/app/uploads \
  ghcr.io/your-username/needadrop:latest
```

### Security Features
- Multi-stage builds with minimal Alpine base
- Non-root user execution
- Read-only filesystem
- Health checks and proper signal handling
- Vulnerability scanning with Trivy

## 📚 API Reference

### Public Endpoints
- `GET /upload/{token}` - Upload form for guests
- `POST /upload/{token}` - File upload handling

### Admin Endpoints
- `GET /admin` - Dashboard
- `GET /admin/links` - Manage upload links
- `GET /admin/uploads` - View all uploads
- `POST /admin/change-password` - Update password

## 📄 License

MIT License - see LICENSE file for details.

---

**Need help?** Open an issue on the repository for support, questions, or contributions.

# Logging System Documentation

## Overview

The NeedADrop application now uses the `tracing` crate for structured logging, providing configurable log levels and detailed context for debugging and monitoring.

## Log Levels

The application supports five log levels (from most to least verbose):

- **TRACE**: Very detailed information, typically only used for debugging
- **DEBUG**: Detailed information useful for debugging
- **INFO**: General information about the application's operation
- **WARN**: Warning messages for potentially problematic situations
- **ERROR**: Error messages for serious problems

## Configuration

### Environment Variable

Set the log level using the `RUST_LOG` environment variable:

```bash
# Set log level to INFO (default)
export RUST_LOG=needadrop=info,info

# Set log level to DEBUG for more detailed output
export RUST_LOG=needadrop=debug,info

# Set log level to WARN to reduce output
export RUST_LOG=needadrop=warn,warn

# Enable debug logging for specific modules
export RUST_LOG=needadrop::handlers=debug,needadrop::database=info,info
```

### Running the Application

```bash
# With default logging (INFO level)
cargo run

# With debug logging
RUST_LOG=debug cargo run

# With minimal logging (errors only)
RUST_LOG=error cargo run
```

## Log Format

The logs include the following information:
- **Timestamp**: When the event occurred
- **Level**: The log level (INFO, DEBUG, etc.)
- **Target**: The module that generated the log
- **File and Line**: Source code location (when available)
- **Thread ID**: The thread that generated the log
- **Structured Data**: Key-value pairs providing context

Example log output:
```
2025-08-13T10:30:45.123456Z  INFO needadrop::handlers: Login attempt username="admin"
2025-08-13T10:30:45.124567Z DEBUG needladrop::database: Found admin user admin_id="123" username="admin"
2025-08-13T10:30:45.125678Z  INFO needadrop::handlers: Password verification successful admin_id="123" username="admin"
```

## Structured Logging

The application uses structured logging with contextual information:

### Authentication Events
- Login attempts with username
- Successful/failed authentications
- Session creation and management

### File Upload Events
- Upload link access and validation
- File processing with size and type information
- Quota checking and updates
- Upload success/failure with detailed context

### Database Operations
- Database initialization and connection
- Query execution with parameters
- Error handling with full context

### File Download Events (New in August 2025)
- Download request authentication checks
- File lookup and validation
- File serving with metadata logging
- Download success/failure tracking

### Example Log Messages

```
INFO needadrop::handlers: File upload initiated token="abc123"
DEBUG needadrop::handlers: Processing uploaded file filename="document.pdf" content_type="application/pdf" link_id="456"
INFO needadrop::handlers: File data read successfully filename="document.pdf" file_size_mb=2.3 link_id="456"
WARN needadrop::handlers: File size exceeds remaining quota filename="large_file.zip" file_size_mb=50.0 remaining_quota_mb=25.0 link_id="456"
DEBUG needadrop::handlers: Found file upload record upload_id="789" original_filename="report.xlsx" stored_filename="uuid.xlsx"
INFO needadrop::handlers: File read successfully upload_id="789" original_filename="report.xlsx" file_size=1024
ERROR needadrop::database: Failed to connect to database database_url="sqlite:needadrop.db"
```

## Benefits

1. **Structured Data**: Key-value pairs make logs easily searchable and parseable
2. **Configurable Levels**: Adjust verbosity based on environment (development vs production)
3. **Performance**: Minimal overhead when debug logging is disabled
4. **Context**: Rich contextual information helps with debugging
5. **Standardized**: Consistent format across the entire application
6. **Security Monitoring**: Authentication and file access events are logged for audit trails
7. **Operational Insights**: File upload metrics and quota usage tracking

## Recent Improvements (August 2025)

- ✅ Enhanced file download logging with authentication tracking
- ✅ Added file access audit trails for security compliance
- ✅ Improved error context in file operations
- ✅ Added structured logging for database operations
- ✅ Updated to latest tracing ecosystem (compatible with Axum 0.8)
- ✅ Enhanced performance with Tower 0.5 service layer improvements
- ✅ Improved logging middleware with Tower-HTTP 0.6

### Framework Compatibility
- **Axum 0.8**: Full tracing integration with improved async logging
- **Tower 0.5**: Enhanced service-level logging and metrics
- **Tower-HTTP 0.6**: Advanced HTTP-specific logging capabilities
- **Tokio 1.47**: Optimized async tracing performance

## Production Recommendations

For production environments:
- Use `RUST_LOG=needadrop=info,warn` for balanced logging
- Consider log aggregation tools like ELK stack or similar
- Monitor ERROR and WARN level messages for system health
- Use DEBUG level temporarily for troubleshooting specific issues

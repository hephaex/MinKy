# Security Policy

## Overview

Minky takes security seriously and implements multiple layers of protection to ensure the safety of user data and prevent common web vulnerabilities.

## Security Features

### Authentication & Authorization

- **JWT-based Authentication**: Secure token-based authentication with configurable expiration
- **Refresh Tokens**: Long-lived refresh tokens for seamless user experience
- **Password Security**: Bcrypt hashing with salt for secure password storage
- **Role-based Access Control**: Users can only access and modify their own documents
- **Optional Authentication**: Public documents accessible without authentication

### Input Validation & Sanitization

- **XSS Prevention**: All user inputs are sanitized using the bleach library
- **Email Validation**: Proper email format validation using email-validator
- **Password Complexity**: Enforced password requirements (minimum length, letters, numbers)
- **Username Validation**: Alphanumeric usernames with underscores only

### Database Security

- **SQL Injection Prevention**: SQLAlchemy ORM with parameterized queries
- **Connection Security**: Secure database connections with environment-based configuration
- **Data Validation**: Server-side validation for all data inputs

### API Security

- **CORS Configuration**: Properly configured Cross-Origin Resource Sharing
- **Rate Limiting**: Can be easily added via Flask-Limiter
- **Input Size Limits**: JSON payload size limitations
- **HTTP Security Headers**: Comprehensive security headers via nginx

### Infrastructure Security

- **Environment Variables**: Sensitive configuration stored in environment variables
- **Docker Security**: Non-root user in Docker containers
- **Health Checks**: Monitoring endpoints for system health
- **Logging**: Structured logging for security monitoring

## Security Headers

The following security headers are implemented:

```
X-Frame-Options: SAMEORIGIN
X-XSS-Protection: 1; mode=block
X-Content-Type-Options: nosniff
Referrer-Policy: no-referrer-when-downgrade
Content-Security-Policy: default-src 'self' http: https: data: blob: 'unsafe-inline'
```

## Reporting Security Vulnerabilities

If you discover a security vulnerability, please follow these steps:

1. **Do not** create a public GitHub issue
2. Send a detailed report to the project maintainers
3. Include steps to reproduce the vulnerability
4. Allow time for the issue to be addressed before public disclosure

## Security Checklist for Deployment

Before deploying to production, ensure:

- [ ] Change all default secret keys in environment variables
- [ ] Use strong, unique passwords for database accounts
- [ ] Enable HTTPS/TLS encryption
- [ ] Configure firewall rules to restrict database access
- [ ] Regularly update dependencies for security patches
- [ ] Enable database backups with encryption
- [ ] Monitor application logs for suspicious activity
- [ ] Implement rate limiting on API endpoints
- [ ] Use a reverse proxy (nginx) with security headers
- [ ] Regularly rotate JWT secret keys

## Dependencies Security

Regularly update dependencies to patch security vulnerabilities:

```bash
pip install --upgrade -r requirements.txt
npm audit fix  # For frontend dependencies
```

## Best Practices

### For Developers

- Always validate and sanitize user inputs
- Use parameterized queries for database operations
- Implement proper error handling without exposing sensitive information
- Follow the principle of least privilege for user permissions
- Regularly review and update security measures

### For Administrators

- Keep the system and dependencies up to date
- Monitor logs for suspicious activities
- Implement proper backup and disaster recovery procedures
- Use strong authentication methods
- Regularly audit user accounts and permissions

## Compliance

This application implements security measures that align with:

- OWASP Top 10 security risks mitigation
- Basic data protection principles
- Standard web application security practices
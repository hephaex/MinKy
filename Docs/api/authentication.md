# Authentication API

## Overview

The MinKy Authentication API provides JWT-based authentication for secure access to the platform. All authenticated endpoints require a valid JWT token in the Authorization header.

## Table of Contents

- [Login](#login)
- [Register](#register)
- [Refresh Token](#refresh-token)
- [Token Format](#token-format)
- [Error Codes](#error-codes)

---

## Login

Authenticate a user and receive access and refresh tokens.

### Endpoint

```
POST /api/auth/login
```

### Request Body

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| email | string | Yes | User's email address (must be valid email format) |
| password | string | Yes | User's password (minimum 8 characters) |

### Example Request

```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "securepassword123"
  }'
```

### Success Response (200 OK)

```json
{
  "success": true,
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": 1,
    "email": "user@example.com",
    "username": "johndoe",
    "role": "user"
  }
}
```

### Error Responses

| Status | Description |
|--------|-------------|
| 400 | Invalid email format or password too short |
| 401 | Invalid credentials |

---

## Register

Create a new user account.

### Endpoint

```
POST /api/auth/register
```

### Request Body

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| email | string | Yes | Valid email format |
| username | string | Yes | 3-50 characters |
| password | string | Yes | Minimum 8 characters |

### Example Request

```bash
curl -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "newuser@example.com",
    "username": "newuser",
    "password": "securepassword123"
  }'
```

### Success Response (201 Created)

```json
{
  "success": true,
  "access_token": null,
  "refresh_token": null,
  "user": null
}
```

**Note:** After successful registration, users should use the login endpoint to obtain tokens.

### Error Responses

| Status | Description |
|--------|-------------|
| 400 | Validation error (invalid email, username length, password length) |
| 409 | Email or username already exists |

---

## Refresh Token

Obtain a new access token using a valid refresh token.

### Endpoint

```
POST /api/auth/refresh
```

### Request Body

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| refresh_token | string | Yes | Valid refresh token |

### Example Request

```bash
curl -X POST http://localhost:3000/api/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  }'
```

### Success Response (200 OK)

```json
{
  "success": true,
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": null
}
```

### Error Responses

| Status | Description |
|--------|-------------|
| 401 | Invalid or expired refresh token |

---

## Token Format

### Access Token

The access token is a JWT that should be included in the `Authorization` header for all authenticated requests:

```
Authorization: Bearer <access_token>
```

### Token Claims

```json
{
  "sub": "1",
  "email": "user@example.com",
  "role": "user",
  "exp": 1640000000,
  "iat": 1639990000
}
```

| Claim | Description |
|-------|-------------|
| sub | User ID |
| email | User's email |
| role | User role (user, admin) |
| exp | Expiration timestamp |
| iat | Issued at timestamp |

### Token Expiration

| Token Type | Default Expiration |
|------------|-------------------|
| Access Token | 15 minutes |
| Refresh Token | 7 days |

---

## Error Codes

### Validation Errors (400)

```json
{
  "success": false,
  "error": "Invalid email format"
}
```

```json
{
  "success": false,
  "error": "Password must be at least 8 characters"
}
```

```json
{
  "success": false,
  "error": "Username must be 3-50 characters"
}
```

### Authentication Errors (401)

```json
{
  "success": false,
  "error": "Authentication required"
}
```

```json
{
  "success": false,
  "error": "Invalid credentials"
}
```

---

## Security Best Practices

1. **Store tokens securely** - Use HttpOnly cookies or secure storage mechanisms
2. **Refresh tokens proactively** - Refresh access tokens before they expire
3. **Handle token errors** - Implement proper error handling for expired/invalid tokens
4. **Use HTTPS** - Always use HTTPS in production
5. **Implement logout** - Clear tokens on user logout

---

## Example: Complete Authentication Flow

```javascript
// 1. Register new user
const registerResponse = await fetch('/api/auth/register', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'user@example.com',
    username: 'johndoe',
    password: 'securepassword123'
  })
});

// 2. Login
const loginResponse = await fetch('/api/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'user@example.com',
    password: 'securepassword123'
  })
});
const { access_token, refresh_token } = await loginResponse.json();

// 3. Use access token for authenticated requests
const documentsResponse = await fetch('/api/documents', {
  headers: {
    'Authorization': `Bearer ${access_token}`,
    'Content-Type': 'application/json'
  }
});

// 4. Refresh token when access token expires
const refreshResponse = await fetch('/api/auth/refresh', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ refresh_token })
});
const { access_token: newAccessToken } = await refreshResponse.json();
```

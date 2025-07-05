# Minky API Documentation

## Overview

The Minky API provides endpoints for managing markdown documents with user authentication and authorization. All endpoints return JSON responses and use standard HTTP status codes.

## Base URL

Development: `http://localhost:5000/api`
Production: `https://your-domain.com/api`

## Authentication

The API uses JWT (JSON Web Tokens) for authentication. Include the token in the Authorization header:

```
Authorization: Bearer <your-jwt-token>
```

## Authentication Endpoints

### Register User

**POST** `/auth/register`

Create a new user account.

**Request Body:**
```json
{
  "username": "johndoe",
  "email": "john@example.com",
  "password": "securepassword123",
  "full_name": "John Doe"
}
```

**Response (201):**
```json
{
  "message": "User registered successfully",
  "user": {
    "id": 1,
    "username": "johndoe",
    "full_name": "John Doe",
    "is_active": true,
    "created_at": "2024-01-01T12:00:00Z"
  },
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
}
```

### Login User

**POST** `/auth/login`

Authenticate a user and receive tokens.

**Request Body:**
```json
{
  "username": "johndoe",  // Can be username or email
  "password": "securepassword123"
}
```

**Response (200):**
```json
{
  "message": "Login successful",
  "user": {
    "id": 1,
    "username": "johndoe",
    "full_name": "John Doe"
  },
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
}
```

### Refresh Token

**POST** `/auth/refresh`

Refresh an access token using a refresh token.

**Headers:**
```
Authorization: Bearer <refresh-token>
```

**Response (200):**
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "user": {
    "id": 1,
    "username": "johndoe",
    "full_name": "John Doe"
  }
}
```

### Get Current User

**GET** `/auth/me`

Get current user profile information.

**Headers:**
```
Authorization: Bearer <access-token>
```

**Response (200):**
```json
{
  "user": {
    "id": 1,
    "username": "johndoe",
    "email": "john@example.com",
    "full_name": "John Doe",
    "is_active": true,
    "is_admin": false,
    "created_at": "2024-01-01T12:00:00Z",
    "updated_at": "2024-01-01T12:00:00Z"
  }
}
```

## Document Endpoints

### Create Document

**POST** `/documents`

Create a new document. Authentication is optional - anonymous users can create public documents.

**Headers (optional):**
```
Authorization: Bearer <access-token>
```

**Request Body:**
```json
{
  "title": "My Document",
  "markdown_content": "# Hello World\n\nThis is my document content.",
  "author": "John Doe",
  "is_public": true
}
```

**Response (201):**
```json
{
  "id": 1,
  "title": "My Document",
  "author": "John Doe",
  "markdown_content": "# Hello World\n\nThis is my document content.",
  "html_content": "<h1>Hello World</h1><p>This is my document content.</p>",
  "user_id": 1,
  "is_public": true,
  "created_at": "2024-01-01T12:00:00Z",
  "updated_at": "2024-01-01T12:00:00Z",
  "owner": {
    "id": 1,
    "username": "johndoe",
    "full_name": "John Doe"
  }
}
```

### List Documents

**GET** `/documents`

List documents with pagination and search. Authentication is optional.

**Query Parameters:**
- `page` (int): Page number (default: 1)
- `per_page` (int): Items per page (default: 10)
- `search` (string): Search query
- `include_private` (boolean): Include private documents (requires authentication)

**Headers (optional):**
```
Authorization: Bearer <access-token>
```

**Response (200):**
```json
{
  "documents": [
    {
      "id": 1,
      "title": "My Document",
      "author": "John Doe",
      "markdown_content": "# Hello World...",
      "html_content": "<h1>Hello World</h1>...",
      "user_id": 1,
      "is_public": true,
      "created_at": "2024-01-01T12:00:00Z",
      "updated_at": "2024-01-01T12:00:00Z",
      "owner": {
        "id": 1,
        "username": "johndoe",
        "full_name": "John Doe"
      }
    }
  ],
  "pagination": {
    "page": 1,
    "per_page": 10,
    "total": 25,
    "pages": 3,
    "has_next": true,
    "has_prev": false
  },
  "search_query": "",
  "include_private": false
}
```

### Get Document

**GET** `/documents/{id}`

Get a specific document by ID. Authentication is optional for public documents.

**Headers (optional):**
```
Authorization: Bearer <access-token>
```

**Response (200):**
```json
{
  "id": 1,
  "title": "My Document",
  "author": "John Doe",
  "markdown_content": "# Hello World\n\nThis is my document content.",
  "html_content": "<h1>Hello World</h1><p>This is my document content.</p>",
  "user_id": 1,
  "is_public": true,
  "created_at": "2024-01-01T12:00:00Z",
  "updated_at": "2024-01-01T12:00:00Z",
  "owner": {
    "id": 1,
    "username": "johndoe",
    "full_name": "John Doe"
  }
}
```

### Update Document

**PUT** `/documents/{id}`

Update a document. Only the document owner can update their documents.

**Headers:**
```
Authorization: Bearer <access-token>
```

**Request Body:**
```json
{
  "title": "Updated Document Title",
  "markdown_content": "# Updated Content\n\nThis is updated content.",
  "author": "Updated Author",
  "is_public": false
}
```

**Response (200):**
```json
{
  "id": 1,
  "title": "Updated Document Title",
  "author": "Updated Author",
  "markdown_content": "# Updated Content\n\nThis is updated content.",
  "html_content": "<h1>Updated Content</h1><p>This is updated content.</p>",
  "user_id": 1,
  "is_public": false,
  "created_at": "2024-01-01T12:00:00Z",
  "updated_at": "2024-01-01T13:00:00Z",
  "owner": {
    "id": 1,
    "username": "johndoe",
    "full_name": "John Doe"
  }
}
```

### Delete Document

**DELETE** `/documents/{id}`

Delete a document. Only the document owner can delete their documents.

**Headers:**
```
Authorization: Bearer <access-token>
```

**Response (200):**
```json
{
  "message": "Document deleted successfully"
}
```

## Health Check Endpoints

### Basic Health Check

**GET** `/health`

Basic health check for monitoring.

**Response (200):**
```json
{
  "status": "healthy",
  "service": "minky-api",
  "database": "connected"
}
```

### Detailed Health Check

**GET** `/health/detailed`

Detailed health information including statistics.

**Response (200):**
```json
{
  "status": "healthy",
  "service": "minky-api",
  "database": {
    "status": "connected",
    "documents": 150,
    "users": 25
  },
  "version": "1.0.0"
}
```

## Error Responses

All endpoints may return error responses in the following format:

**400 Bad Request:**
```json
{
  "error": "Title and markdown_content are required"
}
```

**401 Unauthorized:**
```json
{
  "error": "Invalid credentials"
}
```

**403 Forbidden:**
```json
{
  "error": "Access denied"
}
```

**404 Not Found:**
```json
{
  "error": "Document not found"
}
```

**409 Conflict:**
```json
{
  "error": "Username already exists"
}
```

**500 Internal Server Error:**
```json
{
  "error": "Internal server error"
}
```

## Rate Limiting

API endpoints may be rate limited. When rate limits are exceeded, you'll receive a 429 status code with headers indicating the limit and reset time.

## Search

The search functionality supports:
- Full-text search across document titles and content
- PostgreSQL's built-in text search with ranking
- Partial word matching
- Case-insensitive search
- Search result highlighting in the frontend

## Security

- All user inputs are sanitized to prevent XSS attacks
- SQL injection prevention through SQLAlchemy ORM
- JWT tokens have configurable expiration times
- Password hashing uses bcrypt with salt
- Optional authentication allows public document access
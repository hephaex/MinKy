# MinKy Security Review Report

**Date:** 2026-02-20
**Reviewer:** Security-Reviewer Agent
**Scope:** Backend (Rust), Frontend (React), API Endpoints, Authentication/Authorization

---

## Executive Summary

| Category | Status | Score |
|----------|--------|-------|
| Critical Issues | 3 Found | 🔴 |
| High Issues | 2 Found | 🟡 |
| Medium Issues | 3 Found | 🟡 |
| Dependencies | Clean | ✅ |
| Secrets Management | Excellent | ✅ |
| Input Validation | Good | ✅ |

**Risk Level:** 🔴 **HIGH** - Must fix critical authorization bypass issues before production

---

## Critical Issues (Fix Immediately)

### 1. Missing Authorization on GET Document Endpoint
**Severity:** CRITICAL
**Category:** Broken Access Control (OWASP #5)
**Location:** `minky-rust/src/routes/documents.rs:181-206`

**Issue:**
The `get_document()` function has NO authentication requirement and NO authorization check. Any unauthenticated user can access ANY document by ID.

```rust
// ❌ CRITICAL: No auth_user parameter, no ownership check
async fn get_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,  // No AuthUser!
) -> AppResult<Json<SingleResponse<DocumentResponse>>> {
    let doc: Option<DocumentResponse> = sqlx::query_as(
        r#"SELECT id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at
           FROM documents
           WHERE id = $1"#  // ← Returns ALL documents
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    // No check: is_public? is user_id == current user?
}
```

**Impact:**
- Any user can retrieve any document (private or public)
- Confidential team knowledge could be exposed
- Compliance violation (data access control failure)

**Remediation:**
```rust
async fn get_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth_user: AuthUser,  // ✅ Require authentication
) -> AppResult<Json<SingleResponse<DocumentResponse>>> {
    let doc: Option<DocumentResponse> = sqlx::query_as(
        r#"SELECT id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at
           FROM documents
           WHERE id = $1
           AND (is_public = true OR user_id = $2)"#  // ✅ Check ownership or public
    )
    .bind(id)
    .bind(auth_user.id)  // ✅ Current user ID
    .fetch_optional(&state.db)
    .await?;

    let doc = doc.ok_or_else(|| AppError::NotFound(...))?;
    Ok(...)
}
```

---

### 2. Missing Authorization on UPDATE Document Endpoint
**Severity:** CRITICAL
**Category:** Broken Access Control (OWASP #5)
**Location:** `minky-rust/src/routes/documents.rs:218-260`

**Issue:**
The `update_document()` function has NO authentication requirement. Any unauthenticated user can modify ANY document.

```rust
// ❌ CRITICAL: No auth_user parameter
async fn update_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateDocumentRequest>,
) -> AppResult<Json<SingleResponse<DocumentResponse>>> {
    // No ownership check - updates document regardless of who owns it
    let doc: DocumentResponse = sqlx::query_as(
        r#"UPDATE documents SET ... WHERE id = $6"#  // ← Updates ANY document
    )
}
```

**Impact:**
- Unauthorized modification of documents
- Data integrity violation
- Potential privilege escalation (users can modify admin documents)

**Remediation:**
```rust
async fn update_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth_user: AuthUser,  // ✅ Require authentication
    Json(payload): Json<UpdateDocumentRequest>,
) -> AppResult<Json<SingleResponse<DocumentResponse>>> {
    let doc: DocumentResponse = sqlx::query_as(
        r#"UPDATE documents
           SET title = COALESCE($1, title),
               content = COALESCE($2, content),
               ...
           WHERE id = $6
           AND user_id = $7"#  // ✅ Verify ownership
    )
    .bind(payload.title.as_deref())
    .bind(payload.content.as_deref())
    // ... other bindings ...
    .bind(auth_user.id)  // ✅ Only owner can update
    .fetch_one(&state.db)
    .await?;

    Ok(Json(SingleResponse { success: true, data: doc }))
}
```

---

### 3. Missing Authorization on DELETE Document Endpoint
**Severity:** CRITICAL
**Category:** Broken Access Control (OWASP #5)
**Location:** `minky-rust/src/routes/documents.rs:268-285`

**Issue:**
The `delete_document()` function has NO authentication requirement. Any unauthenticated user can delete ANY document.

```rust
// ❌ CRITICAL: No auth_user parameter
async fn delete_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<DeleteResponse>> {
    let result = sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;  // ← Deletes ANY document
}
```

**Impact:**
- Permanent data loss (documents can be deleted by anyone)
- Denial of service
- Data destruction attack vector

**Remediation:**
```rust
async fn delete_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth_user: AuthUser,  // ✅ Require authentication
) -> AppResult<Json<DeleteResponse>> {
    let result = sqlx::query("DELETE FROM documents WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth_user.id)  // ✅ Only owner can delete
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Document not found or access denied".to_string()));
    }

    Ok(Json(DeleteResponse {
        success: true,
        message: format!("Document {} deleted successfully", id),
    }))
}
```

---

## High Issues (Fix Before Production)

### 1. Frontend Token Stored in localStorage (XSS Vulnerability)
**Severity:** HIGH
**Category:** Sensitive Data Exposure (OWASP #3)
**Location:** `frontend/src/services/api.js:88-98`

**Issue:**
JWT token stored in `localStorage` is vulnerable to XSS attacks.

```javascript
// ❌ HIGH: localStorage vulnerable to XSS
export const authService = {
  login: async (credentials) => {
    const response = await api.post('/auth/login', credentials);
    if (response.data.access_token) {
      localStorage.setItem('token', response.data.access_token);  // ← Vulnerable to XSS
      api.defaults.headers.common['Authorization'] = `Bearer ${response.data.access_token}`;
    }
    return response.data;
  },
};
```

**Code Comment Acknowledged This Issue:**
```javascript
// Line 89-91: SECURITY TODO already noted in codebase
// localStorage is vulnerable to XSS attacks.
// Consider migrating to HttpOnly cookies for token storage.
```

**Impact:**
- If XSS vulnerability exists anywhere in frontend, attacker can steal tokens
- No CSRF protection (token in localStorage)
- No secure flag on token (sent via JavaScript, not HttpOnly)

**Remediation (Backend Required):**
```javascript
// ✅ Backend should set HttpOnly cookies instead
// Set-Cookie: token=<jwt>; HttpOnly; Secure; SameSite=Strict; Path=/

// Frontend then doesn't need to manage token storage
export const authService = {
  login: async (credentials) => {
    const response = await api.post('/auth/login', credentials);
    // Token automatically sent in cookies by axios
    return response.data;
  },
};
```

---

### 2. Document Authorization Bypass via ListQuery Filter
**Severity:** HIGH
**Category:** Broken Access Control (OWASP #5)
**Location:** `minky-rust/src/routes/documents.rs:55-134`

**Issue:**
The `list_documents()` function returns ALL documents (private and public) without verifying user ownership.

```rust
async fn list_documents(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<ListResponse<DocumentResponse>>> {
    // No auth_user, returns all documents
    let documents: Vec<DocumentResponse> = if let Some(ref search) = query.search {
        sqlx::query_as(
            r#"SELECT id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at
               FROM documents
               WHERE title ILIKE '%' || $1 || '%'
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#
        )
        .bind(search)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    };
}
```

**Impact:**
- Unauthenticated users can list/search all documents
- Private documents exposed in search results
- Combined with GET/UPDATE/DELETE bypasses = complete data breach

**Remediation:**
```rust
async fn list_documents(
    State(state): State<AppState>,
    auth_user: AuthUser,  // ✅ Require authentication
    Query(query): Query<ListQuery>,
) -> AppResult<Json<ListResponse<DocumentResponse>>> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    // ✅ Only user's own documents (or public)
    let total: (i64,) = if let Some(ref search) = query.search {
        sqlx::query_as(
            "SELECT COUNT(*) FROM documents
             WHERE (user_id = $1 OR is_public = true)
             AND title ILIKE '%' || $2 || '%'"
        )
        .bind(auth_user.id)
        .bind(search)
        .fetch_one(&state.db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT COUNT(*) FROM documents WHERE user_id = $1 OR is_public = true"
        )
        .bind(auth_user.id)
        .fetch_one(&state.db)
        .await?
    };
    // ... rest of implementation
}
```

---

## Medium Issues (Fix When Possible)

### 1. No CORS Configuration Specified
**Severity:** MEDIUM
**Category:** Security Misconfiguration (OWASP #6)
**Location:** `minky-rust/src/main.rs` (implicitly)

**Issue:**
No CORS configuration found in codebase. If using default (allow all), browsers can't protect against CSRF.

**Recommendation:**
```rust
use tower_http::cors::CorsLayer;

// In create_app()
let cors = CorsLayer::permissive()  // ← Should be restrictive
    .allow_origin("https://yourdomain.com".parse()?)
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_credentials();

let app = Router::new()
    .layer(cors)
    // ... other layers
```

---

### 2. No Rate Limiting on Authentication Endpoints
**Severity:** MEDIUM
**Category:** Insufficient Logging & Monitoring (OWASP #10)
**Location:** `minky-rust/src/routes/auth.rs:46-103`

**Issue:**
Login endpoint can be brute-forced. While failed attempts are logged, no rate limiting exists.

```rust
async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    // No rate limiting
    let user = auth_service.find_user_by_email(&payload.email).await?;
    // Attacker can try 1000s of passwords
}
```

**Recommendation:**
```bash
# Add rate limiting crate
cargo add governor

// Use governor crate for per-IP rate limiting
let limiter = governor::RateLimiter::direct(governor::Quota::per_second(5));

async fn login(...) {
    if limiter.check().is_err() {
        return Err(AppError::TooManyRequests);
    }
    // ... rest of login
}
```

---

### 3. Sensitive Data Potentially Logged
**Severity:** MEDIUM
**Category:** Sensitive Data Exposure (OWASP #3)
**Location:** `minky-rust/src/main.rs:11-12` (logging configuration)

**Issue:**
Debug logging is enabled (`tower_http=debug`). If request/response logging is enabled, JWT tokens could be logged.

```rust
// Line 11-12 of main.rs
.with(
    tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "minky=debug,tower_http=debug".into()),  // ← Debug level
)
```

**Recommendation:**
- Disable `tower_http=debug` in production
- Add log filtering to exclude Authorization headers
- Use environment-specific logging:

```rust
let log_level = if cfg!(debug_assertions) {
    "minky=debug,tower_http=debug".into()
} else {
    "minky=info,tower_http=warn".into()  // Reduced in production
};
```

---

## Positive Security Findings (✅)

### 1. Excellent Secrets Management
**Status:** ✅ EXCELLENT

- JWT secret properly uses `SecretString` (never logged)
- API keys (OpenAI, Anthropic) use `SecretString`
- `config.rs` line 2: `use secrecy::SecretString`
- No hardcoded secrets found in codebase
- `.env` and `.env.example` properly differentiated

```rust
pub struct Config {
    pub jwt_secret: SecretString,  // ✅ Protected
    pub openai_api_key: Option<SecretString>,  // ✅ Protected
    pub anthropic_api_key: Option<SecretString>,  // ✅ Protected
    // ... all sensitive fields use SecretString
}
```

### 2. Strong Password Hashing
**Status:** ✅ EXCELLENT

- Uses Argon2 (modern, resistant to GPU attacks)
- Proper salt generation with OsRng
- No plaintext password storage
- Password never exposed in responses (`#[serde(skip_serializing)]`)

```rust
pub fn hash_password(&self, password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);  // ✅ Cryptographically secure
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;  // ✅ Argon2
    Ok(hash.to_string())
}
```

### 3. SQL Injection Prevention
**Status:** ✅ EXCELLENT

- All queries use parameterized statements (sqlx `$1`, `$2`, etc.)
- No string concatenation in SQL
- Proper input binding throughout

```rust
// ✅ Parameterized query
let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
    .bind(email)  // ✅ Properly bound
    .fetch_optional(&self.db)
    .await?;
```

### 4. Input Validation
**Status:** ✅ GOOD

- Email validation on login/register
- Password length validation (min 8 characters)
- Username validation (3-50 characters)
- Title length validation (1-500 characters)

```rust
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]  // ✅ Email validation
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,  // ✅ Length validation
}
```

### 5. JWT Token Management
**Status:** ✅ GOOD

- Access tokens: 24 hours expiration
- Refresh tokens: 7 days expiration
- Proper claims structure (sub, email, role, exp, iat)
- Token validation on every protected endpoint

```rust
pub fn generate_access_token(&self, user: &User) -> Result<String> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(self.config.jwt_expiration_hours))  // ✅ Expires
        .expect("valid timestamp")
        .timestamp();

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(self.config.jwt_secret.expose_secret().as_bytes()),  // ✅ Secure secret
    )?;
}
```

### 6. Admin Authorization Check
**Status:** ✅ GOOD

- Admin middleware validates role
- Case-sensitive role check (prevents "Admin" bypass)
- Proper authorization extractor for protected routes

```rust
pub async fn admin_middleware(...) -> Result<Response, ...> {
    match auth_service.validate_token(token) {
        Ok(claims) if claims.role == "admin" => Ok(next.run(request).await),  // ✅ Case-sensitive
        Ok(_) => Err((StatusCode::FORBIDDEN, ...)),
        Err(_) => Err((StatusCode::UNAUTHORIZED, ...)),
    }
}
```

### 7. NPM Dependency Security
**Status:** ✅ CLEAN

```
npm audit: found 0 vulnerabilities
```

### 8. Proper Error Handling
**Status:** ✅ GOOD

- Generic error messages (no information disclosure)
- Errors don't expose system details
- Failed login returns generic "Unauthorized"

```rust
// ✅ Generic error messages
Err(AppError::Unauthorized)  // ← Doesn't reveal if email exists

// Instead of:
Err("Email not found in database")  // ❌ Information disclosure
```

---

## Security Checklist

- [x] No hardcoded secrets
- [x] SQL injection prevention (parameterized queries)
- [ ] **Authorization on all endpoints** ❌ CRITICAL
- [x] Strong password hashing (Argon2)
- [x] JWT token validation
- [x] Input validation
- [ ] **XSS prevention (HttpOnly cookies)** ⚠️ HIGH
- [ ] CORS properly configured ⚠️ MEDIUM
- [ ] Rate limiting on auth endpoints ⚠️ MEDIUM
- [ ] Secure logging (no sensitive data) ⚠️ MEDIUM
- [x] Dependencies up to date
- [x] Error messages safe

---

## Recommendations (Priority Order)

### 1. IMMEDIATE (Before Any Production Deployment)
1. **Add `AuthUser` extractor to GET/UPDATE/DELETE document endpoints**
   - Files: `minky-rust/src/routes/documents.rs` lines 181, 218, 268
   - Impact: Prevents data breach from unauthorized access
   - Effort: 15 minutes

2. **Add authorization check to verify document ownership**
   - Only users who own document OR public documents
   - All three endpoints: GET, UPDATE, DELETE
   - Effort: 15 minutes

3. **Test authorization bypass manually**
   - Call DELETE /api/documents/{id} without auth token
   - Should fail, currently succeeds
   - Effort: 5 minutes

### 2. HIGH PRIORITY (Before Handling User Data)
1. **Implement HttpOnly cookie-based token storage**
   - Backend: Set `Set-Cookie: token=...; HttpOnly; Secure; SameSite=Strict`
   - Frontend: Remove localStorage token storage
   - Impact: Prevents XSS token theft
   - Effort: 30 minutes

2. **Configure strict CORS**
   - Only allow requests from your domain
   - Effort: 10 minutes

3. **Add rate limiting to authentication endpoints**
   - Use `governor` crate
   - Limit to 5 login attempts per minute per IP
   - Effort: 20 minutes

### 3. MEDIUM PRIORITY (Hardening)
1. **Reduce logging verbosity in production**
   - Change `tower_http=debug` to `tower_http=warn`
   - Add request header filtering
   - Effort: 15 minutes

2. **Add request logging for audit trail**
   - Log successful logins, document access
   - Track failed operations
   - Effort: 30 minutes

3. **Implement HTTPS enforcement**
   - Redirect HTTP → HTTPS
   - Set HSTS header
   - Effort: 10 minutes

---

## Test Plan for Fixes

```bash
# 1. Test authorization on GET (should fail without auth)
curl -X GET http://localhost:8000/api/documents/{uuid}
# Expected: 401 Unauthorized

# 2. Test with valid token (should work)
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8000/api/documents/{uuid}
# Expected: 200 OK

# 3. Test authorization on UPDATE (should fail without auth)
curl -X PUT http://localhost:8000/api/documents/{uuid} \
  -d '{"title":"Hacked"}' -H "Content-Type: application/json"
# Expected: 401 Unauthorized

# 4. Test ownership check (should fail if not owner)
TOKEN_USER2=$(curl -X POST http://localhost:8000/api/auth/login \
  -d '{"email":"user2@example.com","password":"password"}' | jq -r .access_token)

curl -X PUT http://localhost:8000/api/documents/{user1_doc_uuid} \
  -H "Authorization: Bearer $TOKEN_USER2" \
  -d '{"title":"Hacked"}' -H "Content-Type: application/json"
# Expected: 403 Forbidden or 404 Not Found
```

---

## Summary

| Issue Type | Count | Status |
|-----------|-------|--------|
| Critical Authorization | 3 | 🔴 MUST FIX |
| High Security | 2 | 🟡 HIGH PRIORITY |
| Medium Security | 3 | 🟡 MEDIUM PRIORITY |
| Positive Findings | 8 | ✅ GOOD |

**Overall Status:** 🔴 **HIGH RISK** - The application has excellent secrets management and password security, but **critical authorization bypasses expose all user documents to unauthorized access**. These must be fixed before any production use.

**Estimated Fix Time:** 1.5-2 hours for all critical and high issues

---

## References

- OWASP Top 10 2021: https://owasp.org/Top10/
- JWT Best Practices: https://tools.ietf.org/html/rfc8725
- Secure Token Storage: https://cheatsheetseries.owasp.org/cheatsheets/HTML5_Security_Cheat_Sheet.html
- Authorization Testing: https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/05-Authorization_Testing/README.html

---

**Report Generated:** 2026-02-20
**Next Review Date:** After fixes applied
**Reviewer:** Security-Reviewer Agent

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// User role enumeration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
#[derive(Default)]
pub enum UserRole {
    #[default]
    User,
    Admin,
}


/// User model representing the users table
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: UserRole,
    pub is_active: bool,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for creating a new user
#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub email: String,
    pub username: String,
    pub password: String,
}

/// DTO for updating a user
#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub email: Option<String>,
    pub username: Option<String>,
    pub is_active: Option<bool>,
    pub role: Option<UserRole>,
}

/// Safe user response without sensitive fields
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub email: String,
    pub username: String,
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            username: user.username,
            role: user.role,
            is_active: user.is_active,
            created_at: user.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_user(id: i32, email: &str, role: UserRole, is_active: bool) -> User {
        User {
            id,
            email: email.to_string(),
            username: format!("user{}", id),
            password_hash: "hashed".to_string(),
            role,
            is_active,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_user_role_default_is_user() {
        let role = UserRole::default();
        assert_eq!(role, UserRole::User);
    }

    #[test]
    fn test_user_response_from_user_maps_fields() {
        let user = make_user(42, "alice@example.com", UserRole::User, true);
        let response: UserResponse = user.into();

        assert_eq!(response.id, 42);
        assert_eq!(response.email, "alice@example.com");
        assert_eq!(response.username, "user42");
        assert_eq!(response.role, UserRole::User);
        assert!(response.is_active);
    }

    #[test]
    fn test_user_response_does_not_expose_password() {
        let user = make_user(1, "bob@example.com", UserRole::Admin, true);
        let response: UserResponse = user.into();

        // UserResponse has no password_hash field — verify via serialization
        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("password_hash"));
        assert!(!json.contains("hashed"));
    }

    #[test]
    fn test_user_response_admin_role() {
        let user = make_user(2, "admin@example.com", UserRole::Admin, true);
        let response: UserResponse = user.into();

        assert_eq!(response.role, UserRole::Admin);
    }

    #[test]
    fn test_user_response_inactive_user() {
        let user = make_user(3, "inactive@example.com", UserRole::User, false);
        let response: UserResponse = user.into();

        assert!(!response.is_active);
    }

    #[test]
    fn test_user_role_serde_variants() {
        // UserRole uses default serde (PascalCase) – no rename attribute on the enum
        let user_json = serde_json::to_string(&UserRole::User).unwrap();
        let admin_json = serde_json::to_string(&UserRole::Admin).unwrap();
        // Variants serialize as their Rust names
        assert!(user_json.contains("User"));
        assert!(admin_json.contains("Admin"));
    }

    #[test]
    fn test_user_role_roundtrip() {
        let roles = [UserRole::User, UserRole::Admin];
        for role in &roles {
            let json = serde_json::to_string(role).unwrap();
            let back: UserRole = serde_json::from_str(&json).unwrap();
            assert_eq!(role, &back);
        }
    }

    #[test]
    fn test_user_response_does_not_have_locked_until_field() {
        let user = make_user(10, "locked@example.com", UserRole::User, false);
        let response: UserResponse = user.into();
        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("locked_until"));
        assert!(!json.contains("failed_login_attempts"));
    }

    #[test]
    fn test_create_user_stores_all_fields() {
        let cu = CreateUser {
            email: "new@example.com".to_string(),
            username: "newuser".to_string(),
            password: "secret123".to_string(),
        };
        assert_eq!(cu.email, "new@example.com");
        assert_eq!(cu.username, "newuser");
        assert_eq!(cu.password, "secret123");
    }

    #[test]
    fn test_update_user_all_none_by_default() {
        let uu = UpdateUser {
            email: None,
            username: None,
            is_active: None,
            role: None,
        };
        assert!(uu.email.is_none());
        assert!(uu.username.is_none());
        assert!(uu.is_active.is_none());
        assert!(uu.role.is_none());
    }

    #[test]
    fn test_user_response_created_at_matches_source() {
        use chrono::Utc;
        let now = Utc::now();
        let user = User {
            id: 99,
            email: "time@example.com".to_string(),
            username: "timeuser".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::User,
            is_active: true,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: now,
            updated_at: now,
        };
        let response: UserResponse = user.into();
        assert_eq!(response.id, 99);
        // Timestamps should be equal
        assert_eq!(response.created_at, now);
    }
}

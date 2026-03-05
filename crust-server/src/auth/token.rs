//! JWT token generation and validation

use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// JWT token claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String, // user ID
    pub username: String,
    pub exp: i64,    // expiration timestamp
    pub iat: i64,    // issued at
    pub jti: String, // JWT ID for revocation
}

/// Generate JWT token for a user
pub fn generate_token(
    user_id: &str,
    username: &str,
    secret: &str,
    expires_in_secs: i64,
) -> Result<(String, String), String> {
    let now = Utc::now().timestamp();
    let exp = now + expires_in_secs;
    let jti = uuid::Uuid::new_v4().to_string();

    let claims = TokenClaims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp,
        iat: now,
        jti,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(|e| format!("Token generation failed: {}", e))?;

    Ok((token, claims.jti))
}

/// Validate and decode JWT token
pub fn validate_token(token: &str, secret: &str) -> Result<TokenClaims, String> {
    decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| format!("Token validation failed: {}", e))
}

/// Check if token is expired
#[allow(dead_code)]
pub fn is_token_expired(claims: &TokenClaims) -> bool {
    let now = Utc::now().timestamp();
    claims.exp < now
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation_and_validation() {
        let secret = "test-secret-key-very-long-for-security";
        let user_id = "user-123";
        let username = "testuser";

        let (token, jti) = generate_token(user_id, username, secret, 3600).unwrap();

        assert!(!token.is_empty());
        assert!(!jti.is_empty());

        let claims = validate_token(&token, secret).unwrap();
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.username, username);
        assert_eq!(claims.jti, jti);
        assert!(!is_token_expired(&claims));
    }

    #[test]
    fn test_invalid_token() {
        let secret = "test-secret-key-very-long-for-security";
        let result = validate_token("invalid.token.here", secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_token_expiration() {
        let _secret = "test-secret-key-very-long-for-security";
        let now = Utc::now().timestamp();

        let claims = TokenClaims {
            sub: "user-123".to_string(),
            username: "testuser".to_string(),
            exp: now - 3600, // expired 1 hour ago
            iat: now - 7200,
            jti: "test-jti".to_string(),
        };

        assert!(is_token_expired(&claims));
    }
}

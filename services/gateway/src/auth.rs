//! JWT auth + RBAC for the gateway (ADR 0012).

use axum::http::StatusCode;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// Auth mode from `KELICAD_AUTH_MODE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    /// No tokens required; synthetic editor user.
    Off,
    /// HS256 tokens from `/v1/auth/dev-token`.
    Dev,
    /// Same HS256 validation; issuer must match configured OIDC issuer claim.
    Oidc,
}

impl AuthMode {
    /// Parse from environment.
    pub fn from_env() -> Self {
        match std::env::var("KELICAD_AUTH_MODE")
            .unwrap_or_else(|_| "off".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "dev" => Self::Dev,
            "oidc" => Self::Oidc,
            _ => Self::Off,
        }
    }
}

/// Role hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Read-only.
    Viewer,
    /// Create rooms / append ops.
    Editor,
    /// Audit + admin GDPR.
    Admin,
}

impl Role {
    /// Parse role string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "viewer" => Some(Self::Viewer),
            "editor" => Some(Self::Editor),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }

    /// Rank for comparison.
    pub fn rank(self) -> u8 {
        match self {
            Self::Viewer => 1,
            Self::Editor => 2,
            Self::Admin => 3,
        }
    }

    /// True if `self` meets `required`.
    pub fn allows(self, required: Role) -> bool {
        self.rank() >= required.rank()
    }
}

/// JWT claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject.
    pub sub: String,
    /// Organization id.
    pub org: String,
    /// Role string.
    pub role: String,
    /// Expiry (seconds).
    pub exp: usize,
    /// Issuer.
    pub iss: String,
}

/// Authenticated user extracted from the request.
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// Subject.
    pub sub: String,
    /// Org.
    pub org: String,
    /// Role.
    pub role: Role,
}

impl AuthUser {
    /// Require minimum role.
    pub fn require(&self, min: Role) -> Result<(), StatusCode> {
        if self.role.allows(min) {
            Ok(())
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }
}

/// Shared auth configuration.
#[derive(Clone)]
pub struct AuthConfig {
    /// Mode.
    pub mode: AuthMode,
    /// HS256 secret.
    pub secret: String,
    /// Expected issuer.
    pub issuer: String,
}

impl AuthConfig {
    /// From environment.
    pub fn from_env() -> Self {
        Self {
            mode: AuthMode::from_env(),
            secret: std::env::var("KELICAD_JWT_SECRET")
                .unwrap_or_else(|_| "kelicad-dev-secret-change-me".into()),
            issuer: std::env::var("KELICAD_JWT_ISSUER")
                .unwrap_or_else(|_| "kelicad-dev".into()),
        }
    }

    /// Issue a dev token.
    pub fn issue_token(
        &self,
        sub: &str,
        org: &str,
        role: Role,
        ttl_secs: usize,
    ) -> Result<String, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs() as usize;
        let claims = Claims {
            sub: sub.into(),
            org: org.into(),
            role: match role {
                Role::Viewer => "viewer",
                Role::Editor => "editor",
                Role::Admin => "admin",
            }
            .into(),
            exp: now + ttl_secs,
            iss: self.issuer.clone(),
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| e.to_string())
    }

    /// Decode and validate a bearer token.
    pub fn decode_token(&self, token: &str) -> Result<AuthUser, StatusCode> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[self.issuer.clone()]);
        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
        let role = Role::parse(&data.claims.role).ok_or(StatusCode::UNAUTHORIZED)?;
        Ok(AuthUser {
            sub: data.claims.sub,
            org: data.claims.org,
            role,
        })
    }
}


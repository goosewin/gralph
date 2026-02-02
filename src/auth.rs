//! User authentication module with JWT tokens, password hashing, and rate limiting.
//!
//! This module provides:
//! - User registration with argon2 password hashing
//! - Login with JWT access and refresh tokens
//! - Token validation middleware
//! - Refresh token rotation
//! - Rate limiting on auth endpoints

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// User role for RBAC (to be extended in MC-22).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Developer,
    Viewer,
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::Viewer
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::Developer => write!(f, "developer"),
            UserRole::Viewer => write!(f, "viewer"),
        }
    }
}

/// Stored user representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: u64,
    pub updated_at: u64,
}

impl User {
    /// Create a new user with hashed password.
    pub fn new(email: &str, password: &str, role: UserRole) -> Result<Self, AuthError> {
        let password_hash = hash_password(password)?;
        let now = current_timestamp();
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            email: email.to_lowercase(),
            password_hash,
            role,
            created_at: now,
            updated_at: now,
        })
    }

    /// Verify a password against this user's stored hash.
    pub fn verify_password(&self, password: &str) -> Result<bool, AuthError> {
        verify_password(password, &self.password_hash)
    }
}

/// JWT claims for access tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// User email
    pub email: String,
    /// User role
    pub role: UserRole,
    /// Expiration time (Unix timestamp)
    pub exp: u64,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Token ID for revocation
    pub jti: String,
    /// Token type
    pub token_type: String,
}

/// JWT claims for refresh tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: u64,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Token ID for revocation and rotation tracking
    pub jti: String,
    /// Token type
    pub token_type: String,
    /// Token family for rotation tracking
    pub family: String,
}

/// Token pair returned on successful authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

/// Configuration for JWT tokens.
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for signing tokens
    pub secret: String,
    /// Access token expiration in seconds (default: 15 minutes)
    pub access_token_expiry: u64,
    /// Refresh token expiration in seconds (default: 7 days)
    pub refresh_token_expiry: u64,
    /// Issuer claim
    pub issuer: Option<String>,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: Uuid::new_v4().to_string(),
            access_token_expiry: 15 * 60,        // 15 minutes
            refresh_token_expiry: 7 * 24 * 60 * 60, // 7 days
            issuer: None,
        }
    }
}

impl JwtConfig {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.to_string(),
            ..Default::default()
        }
    }

    pub fn with_expiry(mut self, access_expiry: u64, refresh_expiry: u64) -> Self {
        self.access_token_expiry = access_expiry;
        self.refresh_token_expiry = refresh_expiry;
        self
    }
}

/// Rate limiter configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Window duration in seconds
    pub window_secs: u64,
    /// Lockout duration after exceeding limit (seconds)
    pub lockout_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 5,
            window_secs: 60,
            lockout_secs: 300, // 5 minutes
        }
    }
}

/// Rate limiter entry tracking requests from an IP/identifier.
#[derive(Debug, Clone)]
struct RateLimitEntry {
    requests: Vec<Instant>,
    locked_until: Option<Instant>,
}

impl RateLimitEntry {
    fn new() -> Self {
        Self {
            requests: Vec::new(),
            locked_until: None,
        }
    }
}

/// Rate limiter for authentication endpoints.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    config: RateLimitConfig,
    entries: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a request is allowed for the given identifier (IP address or user ID).
    /// Returns Ok(()) if allowed, Err with remaining lockout time if rate limited.
    pub fn check(&self, identifier: &str) -> Result<(), Duration> {
        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_secs);

        let mut entries = self.entries.write().unwrap();
        let entry = entries.entry(identifier.to_string()).or_insert_with(RateLimitEntry::new);

        // Check if currently locked out
        if let Some(locked_until) = entry.locked_until {
            if now < locked_until {
                return Err(locked_until - now);
            }
            // Lockout expired, reset
            entry.locked_until = None;
            entry.requests.clear();
        }

        // Remove expired requests
        entry.requests.retain(|&req_time| now.duration_since(req_time) < window);

        // Check if over limit
        if entry.requests.len() >= self.config.max_requests as usize {
            let lockout = Duration::from_secs(self.config.lockout_secs);
            entry.locked_until = Some(now + lockout);
            return Err(lockout);
        }

        // Record this request
        entry.requests.push(now);
        Ok(())
    }

    /// Record a failed authentication attempt (may trigger stricter limiting).
    pub fn record_failure(&self, identifier: &str) {
        // For now, failures are counted as regular requests.
        // Could be extended to have stricter handling for failures.
        let _ = self.check(identifier);
    }

    /// Clear rate limit state for an identifier (e.g., after successful login).
    pub fn clear(&self, identifier: &str) {
        let mut entries = self.entries.write().unwrap();
        entries.remove(identifier);
    }

    /// Get the number of remaining requests for an identifier.
    pub fn remaining(&self, identifier: &str) -> u32 {
        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_secs);

        let entries = self.entries.read().unwrap();
        match entries.get(identifier) {
            Some(entry) => {
                if entry.locked_until.is_some() {
                    return 0;
                }
                let valid_requests = entry
                    .requests
                    .iter()
                    .filter(|&&req_time| now.duration_since(req_time) < window)
                    .count();
                self.config.max_requests.saturating_sub(valid_requests as u32)
            }
            None => self.config.max_requests,
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}

/// Refresh token store for tracking valid refresh tokens and rotation.
#[derive(Debug, Clone)]
pub struct RefreshTokenStore {
    /// Maps token family to latest valid token ID
    families: Arc<RwLock<HashMap<String, String>>>,
    /// Revoked token IDs
    revoked: Arc<RwLock<HashMap<String, u64>>>,
}

impl RefreshTokenStore {
    pub fn new() -> Self {
        Self {
            families: Arc::new(RwLock::new(HashMap::new())),
            revoked: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new token in a family.
    pub fn register(&self, family: &str, token_id: &str) {
        let mut families = self.families.write().unwrap();
        families.insert(family.to_string(), token_id.to_string());
    }

    /// Validate a refresh token - returns true if valid and not revoked.
    pub fn validate(&self, family: &str, token_id: &str) -> bool {
        // Check if revoked
        let revoked = self.revoked.read().unwrap();
        if revoked.contains_key(token_id) {
            return false;
        }

        // Check if this is the current token in the family
        let families = self.families.read().unwrap();
        match families.get(family) {
            Some(current_id) => current_id == token_id,
            None => false,
        }
    }

    /// Rotate a token - invalidate the old token and register the new one.
    /// Returns false if the old token was already rotated (potential reuse attack).
    pub fn rotate(&self, family: &str, old_token_id: &str, new_token_id: &str) -> bool {
        let mut families = self.families.write().unwrap();
        let mut revoked = self.revoked.write().unwrap();

        // Check if old token is the current one
        match families.get(family) {
            Some(current_id) if current_id == old_token_id => {
                // Valid rotation
                revoked.insert(old_token_id.to_string(), current_timestamp());
                families.insert(family.to_string(), new_token_id.to_string());
                true
            }
            _ => {
                // Token reuse detected - revoke the entire family
                families.remove(family);
                false
            }
        }
    }

    /// Revoke all tokens in a family (e.g., on logout).
    pub fn revoke_family(&self, family: &str) {
        let mut families = self.families.write().unwrap();
        families.remove(family);
    }

    /// Revoke a specific token.
    pub fn revoke_token(&self, token_id: &str) {
        let mut revoked = self.revoked.write().unwrap();
        revoked.insert(token_id.to_string(), current_timestamp());
    }

    /// Clean up expired revocations (tokens older than max_age seconds).
    pub fn cleanup(&self, max_age: u64) {
        let now = current_timestamp();
        let mut revoked = self.revoked.write().unwrap();
        revoked.retain(|_, &mut revoked_at| now - revoked_at < max_age);
    }
}

impl Default for RefreshTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory user store (to be replaced with database in production).
#[derive(Debug, Clone)]
pub struct UserStore {
    users: Arc<RwLock<HashMap<String, User>>>,
    email_index: Arc<RwLock<HashMap<String, String>>>,
}

impl UserStore {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            email_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new user.
    pub fn register(&self, email: &str, password: &str, role: UserRole) -> Result<User, AuthError> {
        let email_lower = email.to_lowercase();

        // Check if email already exists
        {
            let index = self.email_index.read().unwrap();
            if index.contains_key(&email_lower) {
                return Err(AuthError::EmailAlreadyExists);
            }
        }

        let user = User::new(email, password, role)?;

        {
            let mut users = self.users.write().unwrap();
            let mut index = self.email_index.write().unwrap();
            users.insert(user.id.clone(), user.clone());
            index.insert(email_lower, user.id.clone());
        }

        Ok(user)
    }

    /// Find a user by email.
    pub fn find_by_email(&self, email: &str) -> Option<User> {
        let email_lower = email.to_lowercase();
        let index = self.email_index.read().unwrap();
        let user_id = index.get(&email_lower)?;
        let users = self.users.read().unwrap();
        users.get(user_id).cloned()
    }

    /// Find a user by ID.
    pub fn find_by_id(&self, id: &str) -> Option<User> {
        let users = self.users.read().unwrap();
        users.get(id).cloned()
    }

    /// Update a user.
    pub fn update(&self, user: &User) -> Result<(), AuthError> {
        let mut users = self.users.write().unwrap();
        if !users.contains_key(&user.id) {
            return Err(AuthError::UserNotFound);
        }
        users.insert(user.id.clone(), user.clone());
        Ok(())
    }
}

impl Default for UserStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Authentication error types.
#[derive(Debug, Clone)]
pub enum AuthError {
    InvalidCredentials,
    EmailAlreadyExists,
    UserNotFound,
    PasswordHashError(String),
    TokenGenerationError(String),
    TokenValidationError(String),
    TokenExpired,
    TokenRevoked,
    RateLimited(Duration),
    InvalidEmail,
    WeakPassword(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidCredentials => write!(f, "invalid email or password"),
            AuthError::EmailAlreadyExists => write!(f, "email already registered"),
            AuthError::UserNotFound => write!(f, "user not found"),
            AuthError::PasswordHashError(msg) => write!(f, "password hash error: {}", msg),
            AuthError::TokenGenerationError(msg) => write!(f, "token generation error: {}", msg),
            AuthError::TokenValidationError(msg) => write!(f, "token validation error: {}", msg),
            AuthError::TokenExpired => write!(f, "token has expired"),
            AuthError::TokenRevoked => write!(f, "token has been revoked"),
            AuthError::RateLimited(duration) => {
                write!(f, "rate limited, try again in {} seconds", duration.as_secs())
            }
            AuthError::InvalidEmail => write!(f, "invalid email format"),
            AuthError::WeakPassword(msg) => write!(f, "weak password: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

/// Authentication service combining all auth functionality.
#[derive(Clone)]
pub struct AuthService {
    pub user_store: UserStore,
    pub refresh_store: RefreshTokenStore,
    pub rate_limiter: RateLimiter,
    jwt_config: JwtConfig,
}

impl AuthService {
    pub fn new(jwt_config: JwtConfig) -> Self {
        Self {
            user_store: UserStore::new(),
            refresh_store: RefreshTokenStore::new(),
            rate_limiter: RateLimiter::default(),
            jwt_config,
        }
    }

    pub fn with_rate_limit(mut self, config: RateLimitConfig) -> Self {
        self.rate_limiter = RateLimiter::new(config);
        self
    }

    /// Register a new user.
    pub fn register(
        &self,
        email: &str,
        password: &str,
        ip_address: &str,
    ) -> Result<User, AuthError> {
        // Check rate limit
        self.rate_limiter
            .check(ip_address)
            .map_err(AuthError::RateLimited)?;

        // Validate email format
        if !is_valid_email(email) {
            return Err(AuthError::InvalidEmail);
        }

        // Validate password strength
        validate_password(password)?;

        // Register with default viewer role
        self.user_store.register(email, password, UserRole::Viewer)
    }

    /// Login and return token pair.
    pub fn login(
        &self,
        email: &str,
        password: &str,
        ip_address: &str,
    ) -> Result<TokenPair, AuthError> {
        // Check rate limit
        self.rate_limiter
            .check(ip_address)
            .map_err(AuthError::RateLimited)?;

        // Find user
        let user = self
            .user_store
            .find_by_email(email)
            .ok_or(AuthError::InvalidCredentials)?;

        // Verify password
        if !user.verify_password(password)? {
            self.rate_limiter.record_failure(ip_address);
            return Err(AuthError::InvalidCredentials);
        }

        // Clear rate limit on successful login
        self.rate_limiter.clear(ip_address);

        // Generate tokens
        self.generate_token_pair(&user)
    }

    /// Refresh tokens using a valid refresh token.
    pub fn refresh(&self, refresh_token: &str, ip_address: &str) -> Result<TokenPair, AuthError> {
        // Check rate limit
        self.rate_limiter
            .check(ip_address)
            .map_err(AuthError::RateLimited)?;

        // Decode and validate refresh token
        let claims = decode_refresh_token(refresh_token, &self.jwt_config)?;

        // Validate token in store
        if !self
            .refresh_store
            .validate(&claims.family, &claims.jti)
        {
            return Err(AuthError::TokenRevoked);
        }

        // Find user
        let user = self
            .user_store
            .find_by_id(&claims.sub)
            .ok_or(AuthError::UserNotFound)?;

        // Generate new token pair (without registering - we'll use rotate instead)
        let new_pair = self.generate_token_pair_for_refresh(&user, &claims.family)?;

        // Rotate refresh token
        let new_claims = decode_refresh_token(&new_pair.refresh_token, &self.jwt_config)?;
        if !self
            .refresh_store
            .rotate(&claims.family, &claims.jti, &new_claims.jti)
        {
            // Token reuse detected - potential attack
            return Err(AuthError::TokenRevoked);
        }

        Ok(new_pair)
    }

    /// Validate an access token and return the claims.
    pub fn validate_access_token(&self, token: &str) -> Result<AccessTokenClaims, AuthError> {
        decode_access_token(token, &self.jwt_config)
    }

    /// Logout by revoking the refresh token family.
    pub fn logout(&self, refresh_token: &str) -> Result<(), AuthError> {
        let claims = decode_refresh_token(refresh_token, &self.jwt_config)?;
        self.refresh_store.revoke_family(&claims.family);
        Ok(())
    }

    /// Generate a token pair for a user.
    fn generate_token_pair(&self, user: &User) -> Result<TokenPair, AuthError> {
        let family = Uuid::new_v4().to_string();
        self.generate_token_pair_with_family(user, &family)
    }

    /// Generate a token pair for a user with a specific family.
    fn generate_token_pair_with_family(
        &self,
        user: &User,
        family: &str,
    ) -> Result<TokenPair, AuthError> {
        let pair = self.generate_token_pair_for_refresh(user, family)?;
        // Register refresh token for new token families
        let refresh_claims = decode_refresh_token(&pair.refresh_token, &self.jwt_config)?;
        self.refresh_store.register(family, &refresh_claims.jti);
        Ok(pair)
    }

    /// Generate a token pair for refresh (doesn't register - caller handles rotation).
    fn generate_token_pair_for_refresh(
        &self,
        user: &User,
        family: &str,
    ) -> Result<TokenPair, AuthError> {
        let now = current_timestamp();
        let access_jti = Uuid::new_v4().to_string();
        let refresh_jti = Uuid::new_v4().to_string();

        let access_claims = AccessTokenClaims {
            sub: user.id.clone(),
            email: user.email.clone(),
            role: user.role,
            exp: now + self.jwt_config.access_token_expiry,
            iat: now,
            jti: access_jti,
            token_type: "access".to_string(),
        };

        let refresh_claims = RefreshTokenClaims {
            sub: user.id.clone(),
            exp: now + self.jwt_config.refresh_token_expiry,
            iat: now,
            jti: refresh_jti,
            token_type: "refresh".to_string(),
            family: family.to_string(),
        };

        let access_token = encode_token(&access_claims, &self.jwt_config)?;
        let refresh_token = encode_token(&refresh_claims, &self.jwt_config)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_config.access_token_expiry,
        })
    }
}

// Helper functions

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AuthError::PasswordHashError(e.to_string()))
}

fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| AuthError::PasswordHashError(e.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

fn encode_token<T: Serialize>(claims: &T, config: &JwtConfig) -> Result<String, AuthError> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
    .map_err(|e| AuthError::TokenGenerationError(e.to_string()))
}

fn decode_access_token(token: &str, config: &JwtConfig) -> Result<AccessTokenClaims, AuthError> {
    let mut validation = Validation::default();
    validation.validate_exp = true;

    let token_data: TokenData<AccessTokenClaims> = decode(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
        _ => AuthError::TokenValidationError(e.to_string()),
    })?;

    if token_data.claims.token_type != "access" {
        return Err(AuthError::TokenValidationError(
            "invalid token type".to_string(),
        ));
    }

    Ok(token_data.claims)
}

fn decode_refresh_token(token: &str, config: &JwtConfig) -> Result<RefreshTokenClaims, AuthError> {
    let mut validation = Validation::default();
    validation.validate_exp = true;

    let token_data: TokenData<RefreshTokenClaims> = decode(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
        _ => AuthError::TokenValidationError(e.to_string()),
    })?;

    if token_data.claims.token_type != "refresh" {
        return Err(AuthError::TokenValidationError(
            "invalid token type".to_string(),
        ));
    }

    Ok(token_data.claims)
}

fn is_valid_email(email: &str) -> bool {
    let email = email.trim();
    if email.is_empty() || email.len() > 254 {
        return false;
    }
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let local = parts[0];
    let domain = parts[1];
    if local.is_empty() || local.len() > 64 || domain.is_empty() {
        return false;
    }
    if !domain.contains('.') {
        return false;
    }
    true
}

fn validate_password(password: &str) -> Result<(), AuthError> {
    if password.len() < 8 {
        return Err(AuthError::WeakPassword(
            "password must be at least 8 characters".to_string(),
        ));
    }
    if password.len() > 128 {
        return Err(AuthError::WeakPassword(
            "password must be at most 128 characters".to_string(),
        ));
    }
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    if !has_uppercase || !has_lowercase || !has_digit {
        return Err(AuthError::WeakPassword(
            "password must contain uppercase, lowercase, and digit".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    fn test_jwt_config() -> JwtConfig {
        JwtConfig::new("test-secret-key-for-testing-only")
            .with_expiry(60, 3600) // 1 minute access, 1 hour refresh
    }

    // Password hashing tests

    #[test]
    fn hash_password_produces_unique_hashes() {
        let hash1 = hash_password("password123").unwrap();
        let hash2 = hash_password("password123").unwrap();
        assert_ne!(hash1, hash2, "same password should produce different hashes due to salt");
    }

    #[test]
    fn verify_password_returns_true_for_correct_password() {
        let hash = hash_password("MyPassword1").unwrap();
        assert!(verify_password("MyPassword1", &hash).unwrap());
    }

    #[test]
    fn verify_password_returns_false_for_incorrect_password() {
        let hash = hash_password("MyPassword1").unwrap();
        assert!(!verify_password("WrongPassword1", &hash).unwrap());
    }

    #[test]
    fn verify_password_returns_error_for_invalid_hash() {
        let result = verify_password("password", "not-a-valid-hash");
        assert!(result.is_err());
    }

    // Email validation tests

    #[test]
    fn is_valid_email_accepts_valid_emails() {
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("user.name@example.com"));
        assert!(is_valid_email("user+tag@example.co.uk"));
        assert!(is_valid_email("a@b.co"));
    }

    #[test]
    fn is_valid_email_rejects_invalid_emails() {
        assert!(!is_valid_email(""));
        assert!(!is_valid_email("user"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("user@example"));
        assert!(!is_valid_email("user@@example.com"));
    }

    // Password validation tests

    #[test]
    fn validate_password_accepts_strong_passwords() {
        assert!(validate_password("Password1").is_ok());
        assert!(validate_password("MyStr0ngP@ss").is_ok());
        assert!(validate_password("Abcdefg1").is_ok());
    }

    #[test]
    fn validate_password_rejects_short_passwords() {
        let result = validate_password("Pass1");
        assert!(matches!(result, Err(AuthError::WeakPassword(_))));
    }

    #[test]
    fn validate_password_rejects_missing_uppercase() {
        let result = validate_password("password1");
        assert!(matches!(result, Err(AuthError::WeakPassword(_))));
    }

    #[test]
    fn validate_password_rejects_missing_lowercase() {
        let result = validate_password("PASSWORD1");
        assert!(matches!(result, Err(AuthError::WeakPassword(_))));
    }

    #[test]
    fn validate_password_rejects_missing_digit() {
        let result = validate_password("Passwordx");
        assert!(matches!(result, Err(AuthError::WeakPassword(_))));
    }

    // User tests

    #[test]
    fn user_new_creates_user_with_hashed_password() {
        let user = User::new("test@example.com", "Password1", UserRole::Developer).unwrap();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.role, UserRole::Developer);
        assert!(!user.password_hash.is_empty());
        assert!(user.verify_password("Password1").unwrap());
    }

    #[test]
    fn user_email_is_lowercase() {
        let user = User::new("Test@Example.COM", "Password1", UserRole::Viewer).unwrap();
        assert_eq!(user.email, "test@example.com");
    }

    // UserStore tests

    #[test]
    fn user_store_register_creates_user() {
        let store = UserStore::new();
        let user = store.register("test@example.com", "Password1", UserRole::Viewer).unwrap();
        assert_eq!(user.email, "test@example.com");
    }

    #[test]
    fn user_store_register_rejects_duplicate_email() {
        let store = UserStore::new();
        store.register("test@example.com", "Password1", UserRole::Viewer).unwrap();
        let result = store.register("test@example.com", "Password2", UserRole::Viewer);
        assert!(matches!(result, Err(AuthError::EmailAlreadyExists)));
    }

    #[test]
    fn user_store_register_rejects_duplicate_email_case_insensitive() {
        let store = UserStore::new();
        store.register("test@example.com", "Password1", UserRole::Viewer).unwrap();
        let result = store.register("TEST@EXAMPLE.COM", "Password2", UserRole::Viewer);
        assert!(matches!(result, Err(AuthError::EmailAlreadyExists)));
    }

    #[test]
    fn user_store_find_by_email_returns_user() {
        let store = UserStore::new();
        let created = store.register("test@example.com", "Password1", UserRole::Viewer).unwrap();
        let found = store.find_by_email("test@example.com").unwrap();
        assert_eq!(found.id, created.id);
    }

    #[test]
    fn user_store_find_by_email_is_case_insensitive() {
        let store = UserStore::new();
        store.register("test@example.com", "Password1", UserRole::Viewer).unwrap();
        let found = store.find_by_email("TEST@EXAMPLE.COM");
        assert!(found.is_some());
    }

    #[test]
    fn user_store_find_by_id_returns_user() {
        let store = UserStore::new();
        let created = store.register("test@example.com", "Password1", UserRole::Viewer).unwrap();
        let found = store.find_by_id(&created.id).unwrap();
        assert_eq!(found.email, created.email);
    }

    // JWT token tests

    #[test]
    fn encode_and_decode_access_token() {
        let config = test_jwt_config();
        let claims = AccessTokenClaims {
            sub: "user-123".to_string(),
            email: "test@example.com".to_string(),
            role: UserRole::Developer,
            exp: current_timestamp() + 60,
            iat: current_timestamp(),
            jti: "token-id".to_string(),
            token_type: "access".to_string(),
        };

        let token = encode_token(&claims, &config).unwrap();
        let decoded = decode_access_token(&token, &config).unwrap();

        assert_eq!(decoded.sub, claims.sub);
        assert_eq!(decoded.email, claims.email);
        assert_eq!(decoded.role, claims.role);
    }

    #[test]
    fn decode_access_token_rejects_expired_token() {
        let config = test_jwt_config();
        let claims = AccessTokenClaims {
            sub: "user-123".to_string(),
            email: "test@example.com".to_string(),
            role: UserRole::Developer,
            exp: current_timestamp() - 120, // Already expired (2 minutes ago to account for leeway)
            iat: current_timestamp() - 180,
            jti: "token-id".to_string(),
            token_type: "access".to_string(),
        };

        let token = encode_token(&claims, &config).unwrap();
        let result = decode_access_token(&token, &config);
        assert!(matches!(result, Err(AuthError::TokenExpired)));
    }

    #[test]
    fn encode_and_decode_refresh_token() {
        let config = test_jwt_config();
        let claims = RefreshTokenClaims {
            sub: "user-123".to_string(),
            exp: current_timestamp() + 3600,
            iat: current_timestamp(),
            jti: "token-id".to_string(),
            token_type: "refresh".to_string(),
            family: "family-id".to_string(),
        };

        let token = encode_token(&claims, &config).unwrap();
        let decoded = decode_refresh_token(&token, &config).unwrap();

        assert_eq!(decoded.sub, claims.sub);
        assert_eq!(decoded.family, claims.family);
    }

    // RefreshTokenStore tests

    #[test]
    fn refresh_token_store_validate_returns_true_for_registered_token() {
        let store = RefreshTokenStore::new();
        store.register("family-1", "token-1");
        assert!(store.validate("family-1", "token-1"));
    }

    #[test]
    fn refresh_token_store_validate_returns_false_for_unknown_family() {
        let store = RefreshTokenStore::new();
        assert!(!store.validate("unknown-family", "token-1"));
    }

    #[test]
    fn refresh_token_store_validate_returns_false_for_wrong_token() {
        let store = RefreshTokenStore::new();
        store.register("family-1", "token-1");
        assert!(!store.validate("family-1", "token-2"));
    }

    #[test]
    fn refresh_token_store_rotate_updates_current_token() {
        let store = RefreshTokenStore::new();
        store.register("family-1", "token-1");

        assert!(store.rotate("family-1", "token-1", "token-2"));
        assert!(!store.validate("family-1", "token-1")); // Old token invalid
        assert!(store.validate("family-1", "token-2")); // New token valid
    }

    #[test]
    fn refresh_token_store_rotate_detects_reuse() {
        let store = RefreshTokenStore::new();
        store.register("family-1", "token-1");
        store.rotate("family-1", "token-1", "token-2");

        // Attempting to use old token again should fail and revoke family
        assert!(!store.rotate("family-1", "token-1", "token-3"));
        assert!(!store.validate("family-1", "token-2")); // Entire family revoked
    }

    #[test]
    fn refresh_token_store_revoke_family_invalidates_all() {
        let store = RefreshTokenStore::new();
        store.register("family-1", "token-1");
        store.revoke_family("family-1");
        assert!(!store.validate("family-1", "token-1"));
    }

    // Rate limiter tests

    #[test]
    fn rate_limiter_allows_requests_under_limit() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 3,
            window_secs: 60,
            lockout_secs: 300,
        });

        assert!(limiter.check("ip-1").is_ok());
        assert!(limiter.check("ip-1").is_ok());
        assert!(limiter.check("ip-1").is_ok());
    }

    #[test]
    fn rate_limiter_blocks_requests_over_limit() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 2,
            window_secs: 60,
            lockout_secs: 300,
        });

        assert!(limiter.check("ip-1").is_ok());
        assert!(limiter.check("ip-1").is_ok());
        assert!(limiter.check("ip-1").is_err());
    }

    #[test]
    fn rate_limiter_tracks_different_identifiers_separately() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 1,
            window_secs: 60,
            lockout_secs: 300,
        });

        assert!(limiter.check("ip-1").is_ok());
        assert!(limiter.check("ip-2").is_ok());
        assert!(limiter.check("ip-1").is_err());
        assert!(limiter.check("ip-2").is_err());
    }

    #[test]
    fn rate_limiter_clear_resets_limit() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 1,
            window_secs: 60,
            lockout_secs: 300,
        });

        assert!(limiter.check("ip-1").is_ok());
        assert!(limiter.check("ip-1").is_err());
        limiter.clear("ip-1");
        assert!(limiter.check("ip-1").is_ok());
    }

    #[test]
    fn rate_limiter_remaining_returns_correct_count() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 3,
            window_secs: 60,
            lockout_secs: 300,
        });

        assert_eq!(limiter.remaining("ip-1"), 3);
        limiter.check("ip-1").unwrap();
        assert_eq!(limiter.remaining("ip-1"), 2);
        limiter.check("ip-1").unwrap();
        assert_eq!(limiter.remaining("ip-1"), 1);
    }

    // AuthService tests

    #[test]
    fn auth_service_register_creates_user() {
        let service = AuthService::new(test_jwt_config());
        let user = service.register("test@example.com", "Password1", "127.0.0.1").unwrap();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.role, UserRole::Viewer);
    }

    #[test]
    fn auth_service_register_rejects_invalid_email() {
        let service = AuthService::new(test_jwt_config());
        let result = service.register("not-an-email", "Password1", "127.0.0.1");
        assert!(matches!(result, Err(AuthError::InvalidEmail)));
    }

    #[test]
    fn auth_service_register_rejects_weak_password() {
        let service = AuthService::new(test_jwt_config());
        let result = service.register("test@example.com", "weak", "127.0.0.1");
        assert!(matches!(result, Err(AuthError::WeakPassword(_))));
    }

    #[test]
    fn auth_service_login_returns_tokens() {
        let service = AuthService::new(test_jwt_config());
        service.register("test@example.com", "Password1", "127.0.0.1").unwrap();

        let tokens = service.login("test@example.com", "Password1", "127.0.0.1").unwrap();
        assert!(!tokens.access_token.is_empty());
        assert!(!tokens.refresh_token.is_empty());
        assert_eq!(tokens.token_type, "Bearer");
    }

    #[test]
    fn auth_service_login_rejects_wrong_password() {
        let service = AuthService::new(test_jwt_config());
        service.register("test@example.com", "Password1", "127.0.0.1").unwrap();

        let result = service.login("test@example.com", "WrongPass1", "127.0.0.1");
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[test]
    fn auth_service_login_rejects_unknown_email() {
        let service = AuthService::new(test_jwt_config());
        let result = service.login("unknown@example.com", "Password1", "127.0.0.1");
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[test]
    fn auth_service_validate_access_token_works() {
        let service = AuthService::new(test_jwt_config());
        service.register("test@example.com", "Password1", "127.0.0.1").unwrap();
        let tokens = service.login("test@example.com", "Password1", "127.0.0.1").unwrap();

        let claims = service.validate_access_token(&tokens.access_token).unwrap();
        assert_eq!(claims.email, "test@example.com");
    }

    #[test]
    fn auth_service_refresh_returns_new_tokens() {
        let service = AuthService::new(test_jwt_config());
        service.register("test@example.com", "Password1", "127.0.0.1").unwrap();
        let tokens = service.login("test@example.com", "Password1", "127.0.0.1").unwrap();

        let new_tokens = service.refresh(&tokens.refresh_token, "127.0.0.1").unwrap();
        assert_ne!(new_tokens.access_token, tokens.access_token);
        assert_ne!(new_tokens.refresh_token, tokens.refresh_token);
    }

    #[test]
    fn auth_service_refresh_invalidates_old_token() {
        let service = AuthService::new(test_jwt_config());
        service.register("test@example.com", "Password1", "127.0.0.1").unwrap();
        let tokens = service.login("test@example.com", "Password1", "127.0.0.1").unwrap();
        let _new_tokens = service.refresh(&tokens.refresh_token, "127.0.0.1").unwrap();

        // Old refresh token should no longer work
        let result = service.refresh(&tokens.refresh_token, "127.0.0.1");
        assert!(matches!(result, Err(AuthError::TokenRevoked)));
    }

    #[test]
    fn auth_service_logout_revokes_tokens() {
        let service = AuthService::new(test_jwt_config());
        service.register("test@example.com", "Password1", "127.0.0.1").unwrap();
        let tokens = service.login("test@example.com", "Password1", "127.0.0.1").unwrap();

        service.logout(&tokens.refresh_token).unwrap();

        // Refresh token should no longer work
        let result = service.refresh(&tokens.refresh_token, "127.0.0.1");
        assert!(matches!(result, Err(AuthError::TokenRevoked)));
    }

    #[test]
    fn auth_service_rate_limits_registration() {
        let service = AuthService::new(test_jwt_config())
            .with_rate_limit(RateLimitConfig {
                max_requests: 2,
                window_secs: 60,
                lockout_secs: 300,
            });

        service.register("user1@example.com", "Password1", "127.0.0.1").unwrap();
        service.register("user2@example.com", "Password1", "127.0.0.1").unwrap();
        let result = service.register("user3@example.com", "Password1", "127.0.0.1");
        assert!(matches!(result, Err(AuthError::RateLimited(_))));
    }

    #[test]
    fn auth_service_rate_limits_login() {
        let service = AuthService::new(test_jwt_config())
            .with_rate_limit(RateLimitConfig {
                max_requests: 2,
                window_secs: 60,
                lockout_secs: 300,
            });

        service.register("test@example.com", "Password1", "192.168.1.1").unwrap();

        // Use different IP for login attempts to avoid registration rate limit
        let _ = service.login("test@example.com", "WrongPass1", "127.0.0.2");
        let _ = service.login("test@example.com", "WrongPass1", "127.0.0.2");
        let result = service.login("test@example.com", "Password1", "127.0.0.2");
        assert!(matches!(result, Err(AuthError::RateLimited(_))));
    }

    // UserRole tests

    #[test]
    fn user_role_display_formats_correctly() {
        assert_eq!(format!("{}", UserRole::Admin), "admin");
        assert_eq!(format!("{}", UserRole::Developer), "developer");
        assert_eq!(format!("{}", UserRole::Viewer), "viewer");
    }

    #[test]
    fn user_role_default_is_viewer() {
        assert_eq!(UserRole::default(), UserRole::Viewer);
    }

    // AuthError display tests

    #[test]
    fn auth_error_display_formats_correctly() {
        assert_eq!(
            format!("{}", AuthError::InvalidCredentials),
            "invalid email or password"
        );
        assert_eq!(
            format!("{}", AuthError::EmailAlreadyExists),
            "email already registered"
        );
        assert_eq!(format!("{}", AuthError::TokenExpired), "token has expired");
        assert!(format!("{}", AuthError::RateLimited(StdDuration::from_secs(60)))
            .contains("60 seconds"));
    }
}

//! OpenID Connect (OIDC) integration module.
//!
//! This module provides:
//! - OIDC discovery endpoint handling
//! - Authorization code flow
//! - Token validation (ID tokens and access tokens)
//! - User info retrieval and mapping to local accounts
//!
//! The implementation follows the OpenID Connect Core 1.0 specification.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::auth::{AuthError, TokenPair, User, UserRole, UserStore};

/// OIDC provider configuration.
#[derive(Debug, Clone)]
pub struct OidcConfig {
    /// Client ID registered with the OIDC provider
    pub client_id: String,
    /// Client secret (for confidential clients)
    pub client_secret: Option<String>,
    /// Redirect URI for authorization code callback
    pub redirect_uri: String,
    /// OIDC issuer URL (used for discovery)
    pub issuer: String,
    /// Authorization endpoint (from discovery or manual config)
    pub authorization_endpoint: Option<String>,
    /// Token endpoint (from discovery or manual config)
    pub token_endpoint: Option<String>,
    /// UserInfo endpoint (from discovery or manual config)
    pub userinfo_endpoint: Option<String>,
    /// JWKS URI for token verification (from discovery or manual config)
    pub jwks_uri: Option<String>,
    /// End session endpoint for logout (from discovery or manual config)
    pub end_session_endpoint: Option<String>,
    /// Scopes to request (default: openid profile email)
    pub scopes: Vec<String>,
    /// Claim mapping from OIDC claims to user fields
    pub claim_mapping: OidcClaimMapping,
    /// Response type (default: code)
    pub response_type: String,
    /// Response mode (optional)
    pub response_mode: Option<String>,
    /// Additional authorization parameters
    pub additional_params: HashMap<String, String>,
    /// Whether to use PKCE (Proof Key for Code Exchange)
    pub use_pkce: bool,
    /// Clock skew tolerance in seconds
    pub clock_skew_secs: u64,
}

impl Default for OidcConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: None,
            redirect_uri: String::new(),
            issuer: String::new(),
            authorization_endpoint: None,
            token_endpoint: None,
            userinfo_endpoint: None,
            jwks_uri: None,
            end_session_endpoint: None,
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
            ],
            claim_mapping: OidcClaimMapping::default(),
            response_type: "code".to_string(),
            response_mode: None,
            additional_params: HashMap::new(),
            use_pkce: true,
            clock_skew_secs: 300, // 5 minutes
        }
    }
}

impl OidcConfig {
    pub fn new(client_id: &str, issuer: &str, redirect_uri: &str) -> Self {
        Self {
            client_id: client_id.to_string(),
            issuer: issuer.to_string(),
            redirect_uri: redirect_uri.to_string(),
            ..Default::default()
        }
    }

    pub fn with_client_secret(mut self, secret: &str) -> Self {
        self.client_secret = Some(secret.to_string());
        self
    }

    pub fn with_endpoints(
        mut self,
        auth: &str,
        token: &str,
        userinfo: Option<&str>,
    ) -> Self {
        self.authorization_endpoint = Some(auth.to_string());
        self.token_endpoint = Some(token.to_string());
        self.userinfo_endpoint = userinfo.map(|s| s.to_string());
        self
    }

    pub fn with_jwks_uri(mut self, uri: &str) -> Self {
        self.jwks_uri = Some(uri.to_string());
        self
    }

    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }

    pub fn with_claim_mapping(mut self, mapping: OidcClaimMapping) -> Self {
        self.claim_mapping = mapping;
        self
    }

    pub fn with_pkce(mut self, enabled: bool) -> Self {
        self.use_pkce = enabled;
        self
    }

    pub fn with_additional_params(mut self, params: HashMap<String, String>) -> Self {
        self.additional_params = params;
        self
    }

    /// Validate the configuration is complete.
    pub fn validate(&self) -> Result<(), OidcError> {
        if self.client_id.is_empty() {
            return Err(OidcError::InvalidConfig("client_id is required".to_string()));
        }
        if self.issuer.is_empty() {
            return Err(OidcError::InvalidConfig("issuer is required".to_string()));
        }
        if self.redirect_uri.is_empty() {
            return Err(OidcError::InvalidConfig("redirect_uri is required".to_string()));
        }
        Ok(())
    }
}

/// Mapping from OIDC claims to user profile fields.
#[derive(Debug, Clone)]
pub struct OidcClaimMapping {
    /// Claim name for user identifier (default: sub)
    pub subject: String,
    /// Claim name for email (default: email)
    pub email: String,
    /// Claim name for email verified status (default: email_verified)
    pub email_verified: String,
    /// Claim name for given name (default: given_name)
    pub given_name: Option<String>,
    /// Claim name for family name (default: family_name)
    pub family_name: Option<String>,
    /// Claim name for full name (default: name)
    pub name: Option<String>,
    /// Claim name for preferred username (default: preferred_username)
    pub preferred_username: Option<String>,
    /// Claim name for groups/roles (optional)
    pub groups: Option<String>,
    /// Default role for new users
    pub default_role: UserRole,
    /// Group to role mapping
    pub group_role_mapping: HashMap<String, UserRole>,
}

impl Default for OidcClaimMapping {
    fn default() -> Self {
        Self {
            subject: "sub".to_string(),
            email: "email".to_string(),
            email_verified: "email_verified".to_string(),
            given_name: Some("given_name".to_string()),
            family_name: Some("family_name".to_string()),
            name: Some("name".to_string()),
            preferred_username: Some("preferred_username".to_string()),
            groups: Some("groups".to_string()),
            default_role: UserRole::Viewer,
            group_role_mapping: HashMap::new(),
        }
    }
}

impl OidcClaimMapping {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_email_claim(mut self, claim: &str) -> Self {
        self.email = claim.to_string();
        self
    }

    pub fn with_groups_claim(mut self, claim: &str) -> Self {
        self.groups = Some(claim.to_string());
        self
    }

    pub fn with_default_role(mut self, role: UserRole) -> Self {
        self.default_role = role;
        self
    }

    pub fn with_group_role_mapping(mut self, mapping: HashMap<String, UserRole>) -> Self {
        self.group_role_mapping = mapping;
        self
    }
}

/// OIDC error types.
#[derive(Debug, Clone)]
pub enum OidcError {
    /// Invalid configuration
    InvalidConfig(String),
    /// Discovery failed
    DiscoveryFailed(String),
    /// Authorization request failed
    AuthorizationFailed(String),
    /// Token exchange failed
    TokenExchangeFailed(String),
    /// Token validation failed
    TokenValidationFailed(String),
    /// Token has expired
    TokenExpired,
    /// Invalid state parameter
    InvalidState,
    /// Invalid nonce
    InvalidNonce,
    /// Missing required claim
    MissingClaim(String),
    /// UserInfo retrieval failed
    UserInfoFailed(String),
    /// User provisioning failed
    ProvisioningFailed(String),
    /// PKCE verification failed
    PkceVerificationFailed,
    /// Network error
    NetworkError(String),
}

impl std::fmt::Display for OidcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OidcError::InvalidConfig(msg) => write!(f, "invalid OIDC configuration: {}", msg),
            OidcError::DiscoveryFailed(msg) => write!(f, "OIDC discovery failed: {}", msg),
            OidcError::AuthorizationFailed(msg) => write!(f, "authorization failed: {}", msg),
            OidcError::TokenExchangeFailed(msg) => write!(f, "token exchange failed: {}", msg),
            OidcError::TokenValidationFailed(msg) => write!(f, "token validation failed: {}", msg),
            OidcError::TokenExpired => write!(f, "token has expired"),
            OidcError::InvalidState => write!(f, "invalid state parameter"),
            OidcError::InvalidNonce => write!(f, "invalid nonce in ID token"),
            OidcError::MissingClaim(claim) => write!(f, "missing required claim: {}", claim),
            OidcError::UserInfoFailed(msg) => write!(f, "userinfo retrieval failed: {}", msg),
            OidcError::ProvisioningFailed(msg) => write!(f, "user provisioning failed: {}", msg),
            OidcError::PkceVerificationFailed => write!(f, "PKCE verification failed"),
            OidcError::NetworkError(msg) => write!(f, "network error: {}", msg),
        }
    }
}

impl std::error::Error for OidcError {}

/// OIDC discovery document (OpenID Provider Metadata).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcDiscoveryDocument {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    #[serde(default)]
    pub userinfo_endpoint: Option<String>,
    #[serde(default)]
    pub jwks_uri: Option<String>,
    #[serde(default)]
    pub end_session_endpoint: Option<String>,
    #[serde(default)]
    pub registration_endpoint: Option<String>,
    #[serde(default)]
    pub scopes_supported: Vec<String>,
    #[serde(default)]
    pub response_types_supported: Vec<String>,
    #[serde(default)]
    pub response_modes_supported: Vec<String>,
    #[serde(default)]
    pub grant_types_supported: Vec<String>,
    #[serde(default)]
    pub subject_types_supported: Vec<String>,
    #[serde(default)]
    pub id_token_signing_alg_values_supported: Vec<String>,
    #[serde(default)]
    pub token_endpoint_auth_methods_supported: Vec<String>,
    #[serde(default)]
    pub claims_supported: Vec<String>,
    #[serde(default)]
    pub code_challenge_methods_supported: Vec<String>,
}

/// Authorization request state tracking.
#[derive(Debug, Clone)]
pub struct OidcAuthRequest {
    /// State parameter for CSRF protection
    pub state: String,
    /// Nonce for ID token validation
    pub nonce: String,
    /// PKCE code verifier (if PKCE is enabled)
    pub code_verifier: Option<String>,
    /// Redirect URL after successful auth
    pub redirect_url: Option<String>,
    /// Request creation timestamp
    pub created_at: u64,
    /// Request expiration timestamp
    pub expires_at: u64,
}

impl OidcAuthRequest {
    pub fn new(use_pkce: bool, redirect_url: Option<String>, validity_secs: u64) -> Self {
        let now = current_timestamp();
        Self {
            state: format!("oidc_state_{}", Uuid::new_v4()),
            nonce: format!("oidc_nonce_{}", Uuid::new_v4()),
            code_verifier: if use_pkce {
                Some(generate_pkce_verifier())
            } else {
                None
            },
            redirect_url,
            created_at: now,
            expires_at: now + validity_secs,
        }
    }

    pub fn is_expired(&self) -> bool {
        current_timestamp() > self.expires_at
    }

    /// Generate PKCE code challenge from verifier.
    pub fn code_challenge(&self) -> Option<String> {
        self.code_verifier.as_ref().map(|verifier| {
            use sha2::{Sha256, Digest};
            let hash = Sha256::digest(verifier.as_bytes());
            base64_url_encode(&hash)
        })
    }
}

/// Token response from the OIDC provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcTokenResponse {
    pub access_token: String,
    pub token_type: String,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub id_token: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

/// Decoded ID token claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdTokenClaims {
    /// Issuer
    pub iss: String,
    /// Subject (user identifier)
    pub sub: String,
    /// Audience (client ID)
    pub aud: IdTokenAudience,
    /// Expiration time
    pub exp: u64,
    /// Issued at
    pub iat: u64,
    /// Auth time (optional)
    #[serde(default)]
    pub auth_time: Option<u64>,
    /// Nonce (if provided in auth request)
    #[serde(default)]
    pub nonce: Option<String>,
    /// Authorized party (optional)
    #[serde(default)]
    pub azp: Option<String>,
    /// Access token hash (optional)
    #[serde(default)]
    pub at_hash: Option<String>,
    /// Email (standard claim)
    #[serde(default)]
    pub email: Option<String>,
    /// Email verified (standard claim)
    #[serde(default)]
    pub email_verified: Option<bool>,
    /// Name (standard claim)
    #[serde(default)]
    pub name: Option<String>,
    /// Given name (standard claim)
    #[serde(default)]
    pub given_name: Option<String>,
    /// Family name (standard claim)
    #[serde(default)]
    pub family_name: Option<String>,
    /// Preferred username (standard claim)
    #[serde(default)]
    pub preferred_username: Option<String>,
    /// Picture URL (standard claim)
    #[serde(default)]
    pub picture: Option<String>,
    /// Groups (custom claim)
    #[serde(default)]
    pub groups: Option<Vec<String>>,
    /// Additional claims
    #[serde(flatten)]
    pub additional_claims: HashMap<String, serde_json::Value>,
}

/// ID token audience can be a single string or array.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IdTokenAudience {
    Single(String),
    Multiple(Vec<String>),
}

impl IdTokenAudience {
    pub fn contains(&self, client_id: &str) -> bool {
        match self {
            IdTokenAudience::Single(aud) => aud == client_id,
            IdTokenAudience::Multiple(auds) => auds.iter().any(|a| a == client_id),
        }
    }
}

/// User info response from the userinfo endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcUserInfo {
    pub sub: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: Option<bool>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub given_name: Option<String>,
    #[serde(default)]
    pub family_name: Option<String>,
    #[serde(default)]
    pub preferred_username: Option<String>,
    #[serde(default)]
    pub picture: Option<String>,
    #[serde(default)]
    pub groups: Option<Vec<String>>,
    #[serde(flatten)]
    pub additional_claims: HashMap<String, serde_json::Value>,
}

/// User profile extracted from OIDC claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcUserProfile {
    /// User subject (unique identifier from IdP)
    pub subject: String,
    /// User email
    pub email: String,
    /// Whether email is verified
    pub email_verified: bool,
    /// Given/first name
    pub given_name: Option<String>,
    /// Family/last name
    pub family_name: Option<String>,
    /// Full name
    pub name: Option<String>,
    /// Preferred username
    pub preferred_username: Option<String>,
    /// User groups
    pub groups: Vec<String>,
    /// Determined role
    pub role: UserRole,
}

/// OIDC authentication result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcAuthResult {
    /// JWT token pair for the session
    pub tokens: TokenPair,
    /// User profile from OIDC
    pub user: OidcUserProfile,
    /// Redirect URL from original request
    pub redirect_url: Option<String>,
    /// Whether the user was newly provisioned
    pub is_new_user: bool,
    /// OIDC access token (for userinfo/API calls)
    pub oidc_access_token: String,
    /// OIDC refresh token (if provided)
    pub oidc_refresh_token: Option<String>,
}

/// OIDC client implementation.
pub struct OidcClient {
    config: OidcConfig,
    discovery: Option<OidcDiscoveryDocument>,
    pending_requests: Arc<RwLock<HashMap<String, OidcAuthRequest>>>,
    request_validity_secs: u64,
}

impl OidcClient {
    pub fn new(config: OidcConfig) -> Result<Self, OidcError> {
        config.validate()?;
        Ok(Self {
            config,
            discovery: None,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            request_validity_secs: 600, // 10 minutes
        })
    }

    pub fn with_discovery(mut self, doc: OidcDiscoveryDocument) -> Self {
        self.discovery = Some(doc);
        self
    }

    pub fn with_request_validity(mut self, secs: u64) -> Self {
        self.request_validity_secs = secs;
        self
    }

    /// Get the authorization endpoint URL.
    pub fn authorization_endpoint(&self) -> Option<&str> {
        self.config.authorization_endpoint.as_deref()
            .or_else(|| self.discovery.as_ref().map(|d| d.authorization_endpoint.as_str()))
    }

    /// Get the token endpoint URL.
    pub fn token_endpoint(&self) -> Option<&str> {
        self.config.token_endpoint.as_deref()
            .or_else(|| self.discovery.as_ref().map(|d| d.token_endpoint.as_str()))
    }

    /// Get the userinfo endpoint URL.
    pub fn userinfo_endpoint(&self) -> Option<&str> {
        self.config.userinfo_endpoint.as_deref()
            .or_else(|| self.discovery.as_ref().and_then(|d| d.userinfo_endpoint.as_deref()))
    }

    /// Create an authorization URL for initiating the OIDC flow.
    pub fn create_authorization_url(&self, redirect_url: Option<String>) -> Result<(String, OidcAuthRequest), OidcError> {
        let auth_endpoint = self.authorization_endpoint()
            .ok_or_else(|| OidcError::InvalidConfig("authorization_endpoint not configured".to_string()))?;

        let request = OidcAuthRequest::new(
            self.config.use_pkce,
            redirect_url,
            self.request_validity_secs,
        );

        // Store the request
        {
            let mut requests = self.pending_requests.write().unwrap();
            // Clean up expired requests
            requests.retain(|_, req| !req.is_expired());
            requests.insert(request.state.clone(), request.clone());
        }

        // Build authorization URL
        let mut url = format!(
            "{}?response_type={}&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}",
            auth_endpoint,
            urlencoding::encode(&self.config.response_type),
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(&self.config.scopes.join(" ")),
            urlencoding::encode(&request.state),
            urlencoding::encode(&request.nonce),
        );

        // Add PKCE code challenge if enabled
        if let Some(challenge) = request.code_challenge() {
            url.push_str(&format!(
                "&code_challenge={}&code_challenge_method=S256",
                urlencoding::encode(&challenge)
            ));
        }

        // Add response mode if specified
        if let Some(ref mode) = self.config.response_mode {
            url.push_str(&format!("&response_mode={}", urlencoding::encode(mode)));
        }

        // Add additional parameters
        for (key, value) in &self.config.additional_params {
            url.push_str(&format!("&{}={}", urlencoding::encode(key), urlencoding::encode(value)));
        }

        Ok((url, request))
    }

    /// Validate an authorization callback and return the pending request.
    pub fn validate_callback(
        &self,
        state: &str,
        code: &str,
    ) -> Result<(OidcAuthRequest, String), OidcError> {
        // Look up and validate the pending request
        let request = {
            let mut requests = self.pending_requests.write().unwrap();
            let req = requests.remove(state)
                .ok_or(OidcError::InvalidState)?;

            if req.is_expired() {
                return Err(OidcError::InvalidState);
            }

            req
        };

        Ok((request, code.to_string()))
    }

    /// Build token exchange request body.
    pub fn build_token_request(&self, code: &str, request: &OidcAuthRequest) -> Result<TokenRequestParams, OidcError> {
        let token_endpoint = self.token_endpoint()
            .ok_or_else(|| OidcError::InvalidConfig("token_endpoint not configured".to_string()))?;

        let params = TokenRequestParams {
            endpoint: token_endpoint.to_string(),
            grant_type: "authorization_code".to_string(),
            code: code.to_string(),
            redirect_uri: self.config.redirect_uri.clone(),
            client_id: self.config.client_id.clone(),
            client_secret: self.config.client_secret.clone(),
            code_verifier: request.code_verifier.clone(),
        };

        // Verify PKCE if it was used
        if self.config.use_pkce && request.code_verifier.is_none() {
            return Err(OidcError::PkceVerificationFailed);
        }

        Ok(params)
    }

    /// Validate an ID token.
    pub fn validate_id_token(
        &self,
        id_token: &str,
        expected_nonce: &str,
    ) -> Result<IdTokenClaims, OidcError> {
        // Decode the JWT (without signature verification for now - in production use proper JWT library)
        let parts: Vec<&str> = id_token.split('.').collect();
        if parts.len() != 3 {
            return Err(OidcError::TokenValidationFailed("invalid JWT format".to_string()));
        }

        let claims_json = base64_url_decode(parts[1])
            .map_err(|_| OidcError::TokenValidationFailed("invalid base64 in claims".to_string()))?;

        let claims: IdTokenClaims = serde_json::from_slice(&claims_json)
            .map_err(|e| OidcError::TokenValidationFailed(format!("invalid claims JSON: {}", e)))?;

        // Validate issuer
        if claims.iss != self.config.issuer {
            return Err(OidcError::TokenValidationFailed(format!(
                "issuer mismatch: expected {}, got {}",
                self.config.issuer, claims.iss
            )));
        }

        // Validate audience
        if !claims.aud.contains(&self.config.client_id) {
            return Err(OidcError::TokenValidationFailed(
                "client_id not in audience".to_string()
            ));
        }

        // Validate expiration
        let now = current_timestamp();
        if now > claims.exp + self.config.clock_skew_secs {
            return Err(OidcError::TokenExpired);
        }

        // Validate iat is not too far in the future
        if claims.iat > now + self.config.clock_skew_secs {
            return Err(OidcError::TokenValidationFailed(
                "token issued in the future".to_string()
            ));
        }

        // Validate nonce
        if let Some(ref token_nonce) = claims.nonce {
            if token_nonce != expected_nonce {
                return Err(OidcError::InvalidNonce);
            }
        } else {
            // Nonce is required when provided in auth request
            return Err(OidcError::InvalidNonce);
        }

        Ok(claims)
    }

    /// Extract user profile from ID token claims.
    pub fn extract_user_profile(&self, claims: &IdTokenClaims) -> Result<OidcUserProfile, OidcError> {
        let mapping = &self.config.claim_mapping;

        // Get email (required)
        let email = claims.email.clone()
            .or_else(|| {
                claims.additional_claims.get(&mapping.email)
                    .and_then(|v| v.as_str().map(String::from))
            })
            .ok_or_else(|| OidcError::MissingClaim(mapping.email.clone()))?;

        // Get email verified status
        let email_verified = claims.email_verified.unwrap_or(false);

        // Get groups
        let groups = claims.groups.clone()
            .or_else(|| {
                mapping.groups.as_ref().and_then(|g| {
                    claims.additional_claims.get(g).and_then(|v| {
                        if let Some(arr) = v.as_array() {
                            Some(arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        } else {
                            None
                        }
                    })
                })
            })
            .unwrap_or_default();

        // Determine role
        let role = determine_role(&groups, &mapping.group_role_mapping, mapping.default_role);

        Ok(OidcUserProfile {
            subject: claims.sub.clone(),
            email,
            email_verified,
            given_name: claims.given_name.clone(),
            family_name: claims.family_name.clone(),
            name: claims.name.clone(),
            preferred_username: claims.preferred_username.clone(),
            groups,
            role,
        })
    }

    /// Extract user profile from userinfo response.
    pub fn extract_user_profile_from_userinfo(&self, userinfo: &OidcUserInfo) -> Result<OidcUserProfile, OidcError> {
        let mapping = &self.config.claim_mapping;

        // Get email (required)
        let email = userinfo.email.clone()
            .or_else(|| {
                userinfo.additional_claims.get(&mapping.email)
                    .and_then(|v| v.as_str().map(String::from))
            })
            .ok_or_else(|| OidcError::MissingClaim(mapping.email.clone()))?;

        // Get email verified status
        let email_verified = userinfo.email_verified.unwrap_or(false);

        // Get groups
        let groups = userinfo.groups.clone()
            .or_else(|| {
                mapping.groups.as_ref().and_then(|g| {
                    userinfo.additional_claims.get(g).and_then(|v| {
                        if let Some(arr) = v.as_array() {
                            Some(arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        } else {
                            None
                        }
                    })
                })
            })
            .unwrap_or_default();

        // Determine role
        let role = determine_role(&groups, &mapping.group_role_mapping, mapping.default_role);

        Ok(OidcUserProfile {
            subject: userinfo.sub.clone(),
            email,
            email_verified,
            given_name: userinfo.given_name.clone(),
            family_name: userinfo.family_name.clone(),
            name: userinfo.name.clone(),
            preferred_username: userinfo.preferred_username.clone(),
            groups,
            role,
        })
    }

    /// Get the issuer URL.
    pub fn issuer(&self) -> &str {
        &self.config.issuer
    }

    /// Get the client ID.
    pub fn client_id(&self) -> &str {
        &self.config.client_id
    }

    /// Get the redirect URI.
    pub fn redirect_uri(&self) -> &str {
        &self.config.redirect_uri
    }
}

/// Parameters for token exchange request.
#[derive(Debug, Clone)]
pub struct TokenRequestParams {
    pub endpoint: String,
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub code_verifier: Option<String>,
}

impl TokenRequestParams {
    /// Build form-encoded body for token request.
    pub fn to_form_body(&self) -> String {
        let mut params = vec![
            format!("grant_type={}", urlencoding::encode(&self.grant_type)),
            format!("code={}", urlencoding::encode(&self.code)),
            format!("redirect_uri={}", urlencoding::encode(&self.redirect_uri)),
            format!("client_id={}", urlencoding::encode(&self.client_id)),
        ];

        if let Some(ref secret) = self.client_secret {
            params.push(format!("client_secret={}", urlencoding::encode(secret)));
        }

        if let Some(ref verifier) = self.code_verifier {
            params.push(format!("code_verifier={}", urlencoding::encode(verifier)));
        }

        params.join("&")
    }
}

/// OIDC authentication service combining client with user store.
pub struct OidcAuthService {
    client: OidcClient,
    user_store: UserStore,
    jwt_config: crate::auth::JwtConfig,
}

impl OidcAuthService {
    pub fn new(
        client: OidcClient,
        user_store: UserStore,
        jwt_config: crate::auth::JwtConfig,
    ) -> Self {
        Self {
            client,
            user_store,
            jwt_config,
        }
    }

    /// Start OIDC authentication flow by returning authorization URL.
    pub fn start_auth(&self, redirect_url: Option<String>) -> Result<(String, OidcAuthRequest), OidcError> {
        self.client.create_authorization_url(redirect_url)
    }

    /// Complete OIDC authentication after token exchange.
    pub fn complete_auth(
        &self,
        token_response: OidcTokenResponse,
        request: &OidcAuthRequest,
    ) -> Result<OidcAuthResult, OidcError> {
        // Validate ID token if present
        let profile = if let Some(ref id_token) = token_response.id_token {
            let claims = self.client.validate_id_token(id_token, &request.nonce)?;
            self.client.extract_user_profile(&claims)?
        } else {
            return Err(OidcError::TokenValidationFailed("no id_token in response".to_string()));
        };

        // Provision or update user
        let (user, is_new_user) = self.provision_user(&profile)?;

        // Generate local JWT tokens
        let tokens = self.generate_tokens(&user)
            .map_err(|e| OidcError::ProvisioningFailed(e.to_string()))?;

        Ok(OidcAuthResult {
            tokens,
            user: profile,
            redirect_url: request.redirect_url.clone(),
            is_new_user,
            oidc_access_token: token_response.access_token,
            oidc_refresh_token: token_response.refresh_token,
        })
    }

    /// Complete OIDC authentication with userinfo data.
    pub fn complete_auth_with_userinfo(
        &self,
        token_response: OidcTokenResponse,
        userinfo: OidcUserInfo,
        request: &OidcAuthRequest,
    ) -> Result<OidcAuthResult, OidcError> {
        let profile = self.client.extract_user_profile_from_userinfo(&userinfo)?;

        // Provision or update user
        let (user, is_new_user) = self.provision_user(&profile)?;

        // Generate local JWT tokens
        let tokens = self.generate_tokens(&user)
            .map_err(|e| OidcError::ProvisioningFailed(e.to_string()))?;

        Ok(OidcAuthResult {
            tokens,
            user: profile,
            redirect_url: request.redirect_url.clone(),
            is_new_user,
            oidc_access_token: token_response.access_token,
            oidc_refresh_token: token_response.refresh_token,
        })
    }

    /// Provision a user from OIDC profile (create or update).
    fn provision_user(&self, profile: &OidcUserProfile) -> Result<(User, bool), OidcError> {
        // Check if user exists
        if let Some(existing) = self.user_store.find_by_email(&profile.email) {
            return Ok((existing, false));
        }

        // Create new user with random password (they'll use OIDC for auth)
        let random_password = format!("OIDC-{}-{}", Uuid::new_v4(), Uuid::new_v4());
        let user = self
            .user_store
            .register(&profile.email, &random_password, profile.role)
            .map_err(|e| OidcError::ProvisioningFailed(e.to_string()))?;

        Ok((user, true))
    }

    /// Generate JWT tokens for a user.
    fn generate_tokens(&self, user: &User) -> Result<TokenPair, AuthError> {
        let now = current_timestamp();
        let access_jti = Uuid::new_v4().to_string();
        let refresh_jti = Uuid::new_v4().to_string();
        let family = Uuid::new_v4().to_string();

        let access_claims = crate::auth::AccessTokenClaims {
            sub: user.id.clone(),
            email: user.email.clone(),
            role: user.role,
            exp: now + self.jwt_config.access_token_expiry,
            iat: now,
            jti: access_jti,
            token_type: "access".to_string(),
        };

        let refresh_claims = crate::auth::RefreshTokenClaims {
            sub: user.id.clone(),
            exp: now + self.jwt_config.refresh_token_expiry,
            iat: now,
            jti: refresh_jti,
            token_type: "refresh".to_string(),
            family,
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

    /// Get the authorization endpoint URL.
    pub fn authorization_endpoint(&self) -> Option<&str> {
        self.client.authorization_endpoint()
    }

    /// Get the token endpoint URL.
    pub fn token_endpoint(&self) -> Option<&str> {
        self.client.token_endpoint()
    }

    /// Get the userinfo endpoint URL.
    pub fn userinfo_endpoint(&self) -> Option<&str> {
        self.client.userinfo_endpoint()
    }

    /// Build token exchange request parameters.
    pub fn build_token_request(&self, code: &str, request: &OidcAuthRequest) -> Result<TokenRequestParams, OidcError> {
        self.client.build_token_request(code, request)
    }

    /// Validate a callback from the OIDC provider.
    pub fn validate_callback(&self, state: &str, code: &str) -> Result<(OidcAuthRequest, String), OidcError> {
        self.client.validate_callback(state, code)
    }
}

// Helper functions

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn generate_pkce_verifier() -> String {
    // Generate 32 random bytes and base64url encode
    let random_bytes: [u8; 32] = rand::random();
    base64_url_encode(&random_bytes)
}

fn base64_url_encode(data: &[u8]) -> String {
    BASE64.encode(data)
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string()
}

fn base64_url_decode(data: &str) -> Result<Vec<u8>, base64::DecodeError> {
    let padded = match data.len() % 4 {
        2 => format!("{}==", data),
        3 => format!("{}=", data),
        _ => data.to_string(),
    };
    let standard = padded.replace('-', "+").replace('_', "/");
    BASE64.decode(standard)
}

fn determine_role(
    groups: &[String],
    mapping: &HashMap<String, UserRole>,
    default: UserRole,
) -> UserRole {
    for group in groups {
        if let Some(role) = mapping.get(group) {
            return *role;
        }
    }
    default
}

fn encode_token<T: serde::Serialize>(
    claims: &T,
    config: &crate::auth::JwtConfig,
) -> Result<String, AuthError> {
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        claims,
        &jsonwebtoken::EncodingKey::from_secret(config.secret.as_bytes()),
    )
    .map_err(|e| AuthError::TokenGenerationError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> OidcConfig {
        OidcConfig::new(
            "test-client-id",
            "https://idp.example.com",
            "https://app.example.com/oidc/callback",
        )
        .with_client_secret("test-client-secret")
        .with_endpoints(
            "https://idp.example.com/authorize",
            "https://idp.example.com/token",
            Some("https://idp.example.com/userinfo"),
        )
    }

    // OidcConfig tests

    #[test]
    fn oidc_config_default_values() {
        let config = OidcConfig::default();
        assert!(config.client_id.is_empty());
        assert!(config.issuer.is_empty());
        assert!(config.redirect_uri.is_empty());
        assert!(config.client_secret.is_none());
        assert_eq!(config.scopes, vec!["openid", "profile", "email"]);
        assert_eq!(config.response_type, "code");
        assert!(config.use_pkce);
        assert_eq!(config.clock_skew_secs, 300);
    }

    #[test]
    fn oidc_config_new_sets_required_fields() {
        let config = OidcConfig::new(
            "my-client",
            "https://idp.example.com",
            "https://app.example.com/callback",
        );
        assert_eq!(config.client_id, "my-client");
        assert_eq!(config.issuer, "https://idp.example.com");
        assert_eq!(config.redirect_uri, "https://app.example.com/callback");
    }

    #[test]
    fn oidc_config_builder_methods() {
        let config = OidcConfig::new(
            "my-client",
            "https://idp.example.com",
            "https://app.example.com/callback",
        )
        .with_client_secret("secret123")
        .with_endpoints(
            "https://idp.example.com/authorize",
            "https://idp.example.com/token",
            Some("https://idp.example.com/userinfo"),
        )
        .with_jwks_uri("https://idp.example.com/.well-known/jwks.json")
        .with_scopes(vec!["openid".to_string(), "email".to_string()])
        .with_pkce(false);

        assert_eq!(config.client_secret, Some("secret123".to_string()));
        assert_eq!(config.authorization_endpoint, Some("https://idp.example.com/authorize".to_string()));
        assert_eq!(config.token_endpoint, Some("https://idp.example.com/token".to_string()));
        assert_eq!(config.userinfo_endpoint, Some("https://idp.example.com/userinfo".to_string()));
        assert_eq!(config.jwks_uri, Some("https://idp.example.com/.well-known/jwks.json".to_string()));
        assert_eq!(config.scopes, vec!["openid", "email"]);
        assert!(!config.use_pkce);
    }

    #[test]
    fn oidc_config_validate_missing_client_id() {
        let config = OidcConfig {
            client_id: String::new(),
            issuer: "https://idp.example.com".to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            ..Default::default()
        };
        assert!(matches!(config.validate(), Err(OidcError::InvalidConfig(_))));
    }

    #[test]
    fn oidc_config_validate_missing_issuer() {
        let config = OidcConfig {
            client_id: "my-client".to_string(),
            issuer: String::new(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            ..Default::default()
        };
        assert!(matches!(config.validate(), Err(OidcError::InvalidConfig(_))));
    }

    #[test]
    fn oidc_config_validate_missing_redirect_uri() {
        let config = OidcConfig {
            client_id: "my-client".to_string(),
            issuer: "https://idp.example.com".to_string(),
            redirect_uri: String::new(),
            ..Default::default()
        };
        assert!(matches!(config.validate(), Err(OidcError::InvalidConfig(_))));
    }

    #[test]
    fn oidc_config_validate_success() {
        let config = test_config();
        assert!(config.validate().is_ok());
    }

    // OidcClaimMapping tests

    #[test]
    fn claim_mapping_default_values() {
        let mapping = OidcClaimMapping::default();
        assert_eq!(mapping.subject, "sub");
        assert_eq!(mapping.email, "email");
        assert_eq!(mapping.email_verified, "email_verified");
        assert_eq!(mapping.given_name, Some("given_name".to_string()));
        assert_eq!(mapping.family_name, Some("family_name".to_string()));
        assert_eq!(mapping.name, Some("name".to_string()));
        assert_eq!(mapping.preferred_username, Some("preferred_username".to_string()));
        assert_eq!(mapping.groups, Some("groups".to_string()));
        assert_eq!(mapping.default_role, UserRole::Viewer);
    }

    #[test]
    fn claim_mapping_builder() {
        let mut role_mapping = HashMap::new();
        role_mapping.insert("admins".to_string(), UserRole::Admin);

        let mapping = OidcClaimMapping::new()
            .with_email_claim("mail")
            .with_groups_claim("memberOf")
            .with_default_role(UserRole::Developer)
            .with_group_role_mapping(role_mapping);

        assert_eq!(mapping.email, "mail");
        assert_eq!(mapping.groups, Some("memberOf".to_string()));
        assert_eq!(mapping.default_role, UserRole::Developer);
        assert_eq!(mapping.group_role_mapping.get("admins"), Some(&UserRole::Admin));
    }

    // OidcError display tests

    #[test]
    fn oidc_error_display() {
        assert!(format!("{}", OidcError::InvalidConfig("test".to_string())).contains("configuration"));
        assert!(format!("{}", OidcError::DiscoveryFailed("test".to_string())).contains("discovery"));
        assert!(format!("{}", OidcError::AuthorizationFailed("test".to_string())).contains("authorization"));
        assert!(format!("{}", OidcError::TokenExchangeFailed("test".to_string())).contains("token exchange"));
        assert!(format!("{}", OidcError::TokenValidationFailed("test".to_string())).contains("validation"));
        assert!(format!("{}", OidcError::TokenExpired).contains("expired"));
        assert!(format!("{}", OidcError::InvalidState).contains("state"));
        assert!(format!("{}", OidcError::InvalidNonce).contains("nonce"));
        assert!(format!("{}", OidcError::MissingClaim("email".to_string())).contains("email"));
        assert!(format!("{}", OidcError::UserInfoFailed("test".to_string())).contains("userinfo"));
        assert!(format!("{}", OidcError::ProvisioningFailed("test".to_string())).contains("provisioning"));
        assert!(format!("{}", OidcError::PkceVerificationFailed).contains("PKCE"));
        assert!(format!("{}", OidcError::NetworkError("test".to_string())).contains("network"));
    }

    // OidcAuthRequest tests

    #[test]
    fn oidc_auth_request_new_with_pkce() {
        let request = OidcAuthRequest::new(true, Some("/dashboard".to_string()), 600);
        assert!(request.state.starts_with("oidc_state_"));
        assert!(request.nonce.starts_with("oidc_nonce_"));
        assert!(request.code_verifier.is_some());
        assert_eq!(request.redirect_url, Some("/dashboard".to_string()));
        assert!(!request.is_expired());
    }

    #[test]
    fn oidc_auth_request_new_without_pkce() {
        let request = OidcAuthRequest::new(false, None, 600);
        assert!(request.code_verifier.is_none());
        assert!(request.redirect_url.is_none());
    }

    #[test]
    fn oidc_auth_request_expired() {
        let request = OidcAuthRequest {
            state: "state".to_string(),
            nonce: "nonce".to_string(),
            code_verifier: None,
            redirect_url: None,
            created_at: 0,
            expires_at: 0,
        };
        assert!(request.is_expired());
    }

    #[test]
    fn oidc_auth_request_code_challenge() {
        let request = OidcAuthRequest::new(true, None, 600);
        let challenge = request.code_challenge();
        assert!(challenge.is_some());
        let challenge = challenge.unwrap();
        // SHA256 hash base64url encoded should be 43 chars (256 bits / 6 bits per char)
        assert!(!challenge.is_empty());
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
        assert!(!challenge.contains('='));
    }

    // IdTokenAudience tests

    #[test]
    fn id_token_audience_single_contains() {
        let aud = IdTokenAudience::Single("client-id".to_string());
        assert!(aud.contains("client-id"));
        assert!(!aud.contains("other-client"));
    }

    #[test]
    fn id_token_audience_multiple_contains() {
        let aud = IdTokenAudience::Multiple(vec!["client-1".to_string(), "client-2".to_string()]);
        assert!(aud.contains("client-1"));
        assert!(aud.contains("client-2"));
        assert!(!aud.contains("client-3"));
    }

    // OidcClient tests

    #[test]
    fn oidc_client_new_validates_config() {
        let invalid_config = OidcConfig::default();
        assert!(OidcClient::new(invalid_config).is_err());

        let valid_config = test_config();
        assert!(OidcClient::new(valid_config).is_ok());
    }

    #[test]
    fn oidc_client_endpoints() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        assert_eq!(client.authorization_endpoint(), Some("https://idp.example.com/authorize"));
        assert_eq!(client.token_endpoint(), Some("https://idp.example.com/token"));
        assert_eq!(client.userinfo_endpoint(), Some("https://idp.example.com/userinfo"));
        assert_eq!(client.issuer(), "https://idp.example.com");
        assert_eq!(client.client_id(), "test-client-id");
        assert_eq!(client.redirect_uri(), "https://app.example.com/oidc/callback");
    }

    #[test]
    fn oidc_client_create_authorization_url() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        let (url, request) = client.create_authorization_url(Some("/dashboard".to_string())).unwrap();

        assert!(url.starts_with("https://idp.example.com/authorize?"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=test-client-id"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("scope=openid%20profile%20email"));
        assert!(url.contains("state="));
        assert!(url.contains("nonce="));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
        assert_eq!(request.redirect_url, Some("/dashboard".to_string()));
    }

    #[test]
    fn oidc_client_create_authorization_url_without_pkce() {
        let config = test_config().with_pkce(false);
        let client = OidcClient::new(config).unwrap();

        let (url, request) = client.create_authorization_url(None).unwrap();

        assert!(!url.contains("code_challenge="));
        assert!(!url.contains("code_challenge_method="));
        assert!(request.code_verifier.is_none());
    }

    #[test]
    fn oidc_client_validate_callback_invalid_state() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        let result = client.validate_callback("unknown_state", "auth_code");
        assert!(matches!(result, Err(OidcError::InvalidState)));
    }

    #[test]
    fn oidc_client_validate_callback_valid_state() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        // Create an auth request first
        let (_, request) = client.create_authorization_url(None).unwrap();

        // Validate with the correct state
        let result = client.validate_callback(&request.state, "auth_code");
        assert!(result.is_ok());
        let (returned_request, code) = result.unwrap();
        assert_eq!(returned_request.nonce, request.nonce);
        assert_eq!(code, "auth_code");
    }

    #[test]
    fn oidc_client_build_token_request() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        let request = OidcAuthRequest::new(true, None, 600);
        let token_params = client.build_token_request("auth_code", &request).unwrap();

        assert_eq!(token_params.endpoint, "https://idp.example.com/token");
        assert_eq!(token_params.grant_type, "authorization_code");
        assert_eq!(token_params.code, "auth_code");
        assert_eq!(token_params.redirect_uri, "https://app.example.com/oidc/callback");
        assert_eq!(token_params.client_id, "test-client-id");
        assert_eq!(token_params.client_secret, Some("test-client-secret".to_string()));
        assert!(token_params.code_verifier.is_some());
    }

    #[test]
    fn token_request_params_to_form_body() {
        let params = TokenRequestParams {
            endpoint: "https://idp.example.com/token".to_string(),
            grant_type: "authorization_code".to_string(),
            code: "auth_code".to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            client_id: "my-client".to_string(),
            client_secret: Some("secret".to_string()),
            code_verifier: Some("verifier".to_string()),
        };

        let body = params.to_form_body();
        assert!(body.contains("grant_type=authorization_code"));
        assert!(body.contains("code=auth_code"));
        assert!(body.contains("redirect_uri="));
        assert!(body.contains("client_id=my-client"));
        assert!(body.contains("client_secret=secret"));
        assert!(body.contains("code_verifier=verifier"));
    }

    // ID token validation tests

    #[test]
    fn oidc_client_validate_id_token_invalid_format() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        let result = client.validate_id_token("not.a.valid.jwt.too.many.parts", "nonce");
        assert!(matches!(result, Err(OidcError::TokenValidationFailed(_))));

        let result = client.validate_id_token("only.two", "nonce");
        assert!(matches!(result, Err(OidcError::TokenValidationFailed(_))));
    }

    #[test]
    fn oidc_client_validate_id_token_success() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        // Create a mock ID token with valid claims
        let now = current_timestamp();
        let claims = serde_json::json!({
            "iss": "https://idp.example.com",
            "sub": "user123",
            "aud": "test-client-id",
            "exp": now + 3600,
            "iat": now,
            "nonce": "test-nonce",
            "email": "user@example.com",
            "email_verified": true,
            "name": "Test User"
        });

        let header = base64_url_encode(r#"{"alg":"RS256","typ":"JWT"}"#.as_bytes());
        let payload = base64_url_encode(serde_json::to_string(&claims).unwrap().as_bytes());
        let signature = base64_url_encode(b"fake_signature"); // Not validated in this implementation
        let id_token = format!("{}.{}.{}", header, payload, signature);

        let result = client.validate_id_token(&id_token, "test-nonce");
        assert!(result.is_ok());
        let decoded = result.unwrap();
        assert_eq!(decoded.sub, "user123");
        assert_eq!(decoded.email, Some("user@example.com".to_string()));
    }

    #[test]
    fn oidc_client_validate_id_token_wrong_issuer() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        let now = current_timestamp();
        let claims = serde_json::json!({
            "iss": "https://wrong-idp.example.com",
            "sub": "user123",
            "aud": "test-client-id",
            "exp": now + 3600,
            "iat": now,
            "nonce": "test-nonce"
        });

        let header = base64_url_encode(r#"{"alg":"RS256"}"#.as_bytes());
        let payload = base64_url_encode(serde_json::to_string(&claims).unwrap().as_bytes());
        let id_token = format!("{}.{}.sig", header, payload);

        let result = client.validate_id_token(&id_token, "test-nonce");
        assert!(matches!(result, Err(OidcError::TokenValidationFailed(_))));
    }

    #[test]
    fn oidc_client_validate_id_token_wrong_audience() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        let now = current_timestamp();
        let claims = serde_json::json!({
            "iss": "https://idp.example.com",
            "sub": "user123",
            "aud": "wrong-client-id",
            "exp": now + 3600,
            "iat": now,
            "nonce": "test-nonce"
        });

        let header = base64_url_encode(r#"{"alg":"RS256"}"#.as_bytes());
        let payload = base64_url_encode(serde_json::to_string(&claims).unwrap().as_bytes());
        let id_token = format!("{}.{}.sig", header, payload);

        let result = client.validate_id_token(&id_token, "test-nonce");
        assert!(matches!(result, Err(OidcError::TokenValidationFailed(_))));
    }

    #[test]
    fn oidc_client_validate_id_token_expired() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        let now = current_timestamp();
        let claims = serde_json::json!({
            "iss": "https://idp.example.com",
            "sub": "user123",
            "aud": "test-client-id",
            "exp": now - 600, // Expired 10 minutes ago
            "iat": now - 3600,
            "nonce": "test-nonce"
        });

        let header = base64_url_encode(r#"{"alg":"RS256"}"#.as_bytes());
        let payload = base64_url_encode(serde_json::to_string(&claims).unwrap().as_bytes());
        let id_token = format!("{}.{}.sig", header, payload);

        let result = client.validate_id_token(&id_token, "test-nonce");
        assert!(matches!(result, Err(OidcError::TokenExpired)));
    }

    #[test]
    fn oidc_client_validate_id_token_wrong_nonce() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        let now = current_timestamp();
        let claims = serde_json::json!({
            "iss": "https://idp.example.com",
            "sub": "user123",
            "aud": "test-client-id",
            "exp": now + 3600,
            "iat": now,
            "nonce": "wrong-nonce"
        });

        let header = base64_url_encode(r#"{"alg":"RS256"}"#.as_bytes());
        let payload = base64_url_encode(serde_json::to_string(&claims).unwrap().as_bytes());
        let id_token = format!("{}.{}.sig", header, payload);

        let result = client.validate_id_token(&id_token, "expected-nonce");
        assert!(matches!(result, Err(OidcError::InvalidNonce)));
    }

    #[test]
    fn oidc_client_validate_id_token_missing_nonce() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        let now = current_timestamp();
        let claims = serde_json::json!({
            "iss": "https://idp.example.com",
            "sub": "user123",
            "aud": "test-client-id",
            "exp": now + 3600,
            "iat": now
            // No nonce
        });

        let header = base64_url_encode(r#"{"alg":"RS256"}"#.as_bytes());
        let payload = base64_url_encode(serde_json::to_string(&claims).unwrap().as_bytes());
        let id_token = format!("{}.{}.sig", header, payload);

        let result = client.validate_id_token(&id_token, "expected-nonce");
        assert!(matches!(result, Err(OidcError::InvalidNonce)));
    }

    // User profile extraction tests

    #[test]
    fn oidc_client_extract_user_profile() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        let claims = IdTokenClaims {
            iss: "https://idp.example.com".to_string(),
            sub: "user123".to_string(),
            aud: IdTokenAudience::Single("test-client-id".to_string()),
            exp: current_timestamp() + 3600,
            iat: current_timestamp(),
            auth_time: None,
            nonce: Some("nonce".to_string()),
            azp: None,
            at_hash: None,
            email: Some("user@example.com".to_string()),
            email_verified: Some(true),
            name: Some("Test User".to_string()),
            given_name: Some("Test".to_string()),
            family_name: Some("User".to_string()),
            preferred_username: Some("testuser".to_string()),
            picture: None,
            groups: Some(vec!["developers".to_string()]),
            additional_claims: HashMap::new(),
        };

        let profile = client.extract_user_profile(&claims).unwrap();
        assert_eq!(profile.subject, "user123");
        assert_eq!(profile.email, "user@example.com");
        assert!(profile.email_verified);
        assert_eq!(profile.name, Some("Test User".to_string()));
        assert_eq!(profile.given_name, Some("Test".to_string()));
        assert_eq!(profile.family_name, Some("User".to_string()));
        assert_eq!(profile.preferred_username, Some("testuser".to_string()));
        assert_eq!(profile.groups, vec!["developers"]);
        assert_eq!(profile.role, UserRole::Viewer); // Default role
    }

    #[test]
    fn oidc_client_extract_user_profile_with_role_mapping() {
        let mut role_mapping = HashMap::new();
        role_mapping.insert("admins".to_string(), UserRole::Admin);
        role_mapping.insert("developers".to_string(), UserRole::Developer);

        let config = test_config()
            .with_claim_mapping(OidcClaimMapping::default().with_group_role_mapping(role_mapping));
        let client = OidcClient::new(config).unwrap();

        let claims = IdTokenClaims {
            iss: "https://idp.example.com".to_string(),
            sub: "user123".to_string(),
            aud: IdTokenAudience::Single("test-client-id".to_string()),
            exp: current_timestamp() + 3600,
            iat: current_timestamp(),
            auth_time: None,
            nonce: Some("nonce".to_string()),
            azp: None,
            at_hash: None,
            email: Some("user@example.com".to_string()),
            email_verified: Some(true),
            name: None,
            given_name: None,
            family_name: None,
            preferred_username: None,
            picture: None,
            groups: Some(vec!["developers".to_string()]),
            additional_claims: HashMap::new(),
        };

        let profile = client.extract_user_profile(&claims).unwrap();
        assert_eq!(profile.role, UserRole::Developer);
    }

    #[test]
    fn oidc_client_extract_user_profile_missing_email() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        let claims = IdTokenClaims {
            iss: "https://idp.example.com".to_string(),
            sub: "user123".to_string(),
            aud: IdTokenAudience::Single("test-client-id".to_string()),
            exp: current_timestamp() + 3600,
            iat: current_timestamp(),
            auth_time: None,
            nonce: Some("nonce".to_string()),
            azp: None,
            at_hash: None,
            email: None, // Missing email
            email_verified: None,
            name: None,
            given_name: None,
            family_name: None,
            preferred_username: None,
            picture: None,
            groups: None,
            additional_claims: HashMap::new(),
        };

        let result = client.extract_user_profile(&claims);
        assert!(matches!(result, Err(OidcError::MissingClaim(_))));
    }

    // UserInfo extraction tests

    #[test]
    fn oidc_client_extract_user_profile_from_userinfo() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();

        let userinfo = OidcUserInfo {
            sub: "user123".to_string(),
            email: Some("user@example.com".to_string()),
            email_verified: Some(true),
            name: Some("Test User".to_string()),
            given_name: Some("Test".to_string()),
            family_name: Some("User".to_string()),
            preferred_username: Some("testuser".to_string()),
            picture: Some("https://example.com/avatar.png".to_string()),
            groups: Some(vec!["users".to_string()]),
            additional_claims: HashMap::new(),
        };

        let profile = client.extract_user_profile_from_userinfo(&userinfo).unwrap();
        assert_eq!(profile.subject, "user123");
        assert_eq!(profile.email, "user@example.com");
        assert!(profile.email_verified);
        assert_eq!(profile.name, Some("Test User".to_string()));
    }

    // OidcAuthService tests

    #[test]
    fn oidc_auth_service_start_auth() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();
        let user_store = UserStore::new();
        let jwt_config = crate::auth::JwtConfig::new("test-secret");

        let service = OidcAuthService::new(client, user_store, jwt_config);

        let (url, request) = service.start_auth(Some("/dashboard".to_string())).unwrap();

        assert!(url.contains("authorize"));
        assert!(request.redirect_url.is_some());
    }

    #[test]
    fn oidc_auth_service_endpoints() {
        let config = test_config();
        let client = OidcClient::new(config).unwrap();
        let user_store = UserStore::new();
        let jwt_config = crate::auth::JwtConfig::new("test-secret");

        let service = OidcAuthService::new(client, user_store, jwt_config);

        assert_eq!(service.authorization_endpoint(), Some("https://idp.example.com/authorize"));
        assert_eq!(service.token_endpoint(), Some("https://idp.example.com/token"));
        assert_eq!(service.userinfo_endpoint(), Some("https://idp.example.com/userinfo"));
    }

    // Helper function tests

    #[test]
    fn test_base64_url_encode_decode_roundtrip() {
        let data = b"test data for encoding";
        let encoded = base64_url_encode(data);
        let decoded = base64_url_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base64_url_encode_no_padding() {
        let data = b"abc";
        let encoded = base64_url_encode(data);
        assert!(!encoded.contains('='));
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
    }

    #[test]
    fn test_determine_role() {
        let mut mapping = HashMap::new();
        mapping.insert("admins".to_string(), UserRole::Admin);
        mapping.insert("developers".to_string(), UserRole::Developer);

        assert_eq!(
            determine_role(&["admins".to_string()], &mapping, UserRole::Viewer),
            UserRole::Admin
        );
        assert_eq!(
            determine_role(&["developers".to_string()], &mapping, UserRole::Viewer),
            UserRole::Developer
        );
        assert_eq!(
            determine_role(&["users".to_string()], &mapping, UserRole::Viewer),
            UserRole::Viewer
        );
        assert_eq!(
            determine_role(&[], &mapping, UserRole::Developer),
            UserRole::Developer
        );
    }

    #[test]
    fn test_generate_pkce_verifier() {
        let verifier1 = generate_pkce_verifier();
        let verifier2 = generate_pkce_verifier();

        // Verifiers should be unique
        assert_ne!(verifier1, verifier2);

        // Verifiers should be base64url encoded (no +, /, or =)
        assert!(!verifier1.contains('+'));
        assert!(!verifier1.contains('/'));
        assert!(!verifier1.contains('='));
    }

    // Discovery document tests

    #[test]
    fn discovery_document_deserialization() {
        let json = r#"{
            "issuer": "https://idp.example.com",
            "authorization_endpoint": "https://idp.example.com/authorize",
            "token_endpoint": "https://idp.example.com/token",
            "userinfo_endpoint": "https://idp.example.com/userinfo",
            "jwks_uri": "https://idp.example.com/.well-known/jwks.json",
            "scopes_supported": ["openid", "profile", "email"],
            "response_types_supported": ["code", "token"],
            "code_challenge_methods_supported": ["S256"]
        }"#;

        let doc: OidcDiscoveryDocument = serde_json::from_str(json).unwrap();
        assert_eq!(doc.issuer, "https://idp.example.com");
        assert_eq!(doc.authorization_endpoint, "https://idp.example.com/authorize");
        assert_eq!(doc.token_endpoint, "https://idp.example.com/token");
        assert_eq!(doc.userinfo_endpoint, Some("https://idp.example.com/userinfo".to_string()));
        assert_eq!(doc.jwks_uri, Some("https://idp.example.com/.well-known/jwks.json".to_string()));
        assert!(doc.scopes_supported.contains(&"openid".to_string()));
        assert!(doc.code_challenge_methods_supported.contains(&"S256".to_string()));
    }

    #[test]
    fn discovery_document_minimal() {
        let json = r#"{
            "issuer": "https://idp.example.com",
            "authorization_endpoint": "https://idp.example.com/authorize",
            "token_endpoint": "https://idp.example.com/token"
        }"#;

        let doc: OidcDiscoveryDocument = serde_json::from_str(json).unwrap();
        assert_eq!(doc.issuer, "https://idp.example.com");
        assert!(doc.userinfo_endpoint.is_none());
        assert!(doc.jwks_uri.is_none());
        assert!(doc.scopes_supported.is_empty());
    }

    // Token response tests

    #[test]
    fn token_response_deserialization() {
        let json = r#"{
            "access_token": "access_token_value",
            "token_type": "Bearer",
            "expires_in": 3600,
            "refresh_token": "refresh_token_value",
            "id_token": "id_token_value",
            "scope": "openid profile email"
        }"#;

        let response: OidcTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, "access_token_value");
        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, Some(3600));
        assert_eq!(response.refresh_token, Some("refresh_token_value".to_string()));
        assert_eq!(response.id_token, Some("id_token_value".to_string()));
        assert_eq!(response.scope, Some("openid profile email".to_string()));
    }

    #[test]
    fn token_response_minimal() {
        let json = r#"{
            "access_token": "access_token_value",
            "token_type": "Bearer"
        }"#;

        let response: OidcTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, "access_token_value");
        assert!(response.expires_in.is_none());
        assert!(response.refresh_token.is_none());
        assert!(response.id_token.is_none());
    }
}

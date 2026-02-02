//! OAuth2 social login providers module.
//!
//! This module provides OAuth2 authentication for social providers:
//! - GitHub
//! - Google
//! - GitLab
//!
//! Each provider implements the OAuth2 authorization code flow with support for:
//! - Provider-specific configuration
//! - User profile extraction
//! - Account linking for existing users
//! - Automatic user provisioning for new users

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::auth::{AuthError, TokenPair, User, UserRole, UserStore};

/// Supported OAuth2 providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuth2Provider {
    GitHub,
    Google,
    GitLab,
}

impl std::fmt::Display for OAuth2Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuth2Provider::GitHub => write!(f, "github"),
            OAuth2Provider::Google => write!(f, "google"),
            OAuth2Provider::GitLab => write!(f, "gitlab"),
        }
    }
}

impl std::str::FromStr for OAuth2Provider {
    type Err = OAuth2Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github" => Ok(OAuth2Provider::GitHub),
            "google" => Ok(OAuth2Provider::Google),
            "gitlab" => Ok(OAuth2Provider::GitLab),
            _ => Err(OAuth2Error::UnknownProvider(s.to_string())),
        }
    }
}

/// OAuth2 error types.
#[derive(Debug, Clone)]
pub enum OAuth2Error {
    /// Unknown provider
    UnknownProvider(String),
    /// Invalid configuration
    InvalidConfig(String),
    /// Authorization failed
    AuthorizationFailed(String),
    /// Token exchange failed
    TokenExchangeFailed(String),
    /// User profile retrieval failed
    ProfileFailed(String),
    /// Invalid state parameter
    InvalidState,
    /// Account linking failed
    LinkingFailed(String),
    /// User provisioning failed
    ProvisioningFailed(String),
    /// Provider not configured
    ProviderNotConfigured(OAuth2Provider),
    /// Network error
    NetworkError(String),
}

impl std::fmt::Display for OAuth2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuth2Error::UnknownProvider(name) => write!(f, "unknown OAuth2 provider: {}", name),
            OAuth2Error::InvalidConfig(msg) => write!(f, "invalid OAuth2 configuration: {}", msg),
            OAuth2Error::AuthorizationFailed(msg) => write!(f, "authorization failed: {}", msg),
            OAuth2Error::TokenExchangeFailed(msg) => write!(f, "token exchange failed: {}", msg),
            OAuth2Error::ProfileFailed(msg) => write!(f, "profile retrieval failed: {}", msg),
            OAuth2Error::InvalidState => write!(f, "invalid state parameter"),
            OAuth2Error::LinkingFailed(msg) => write!(f, "account linking failed: {}", msg),
            OAuth2Error::ProvisioningFailed(msg) => write!(f, "user provisioning failed: {}", msg),
            OAuth2Error::ProviderNotConfigured(provider) => {
                write!(f, "provider {} not configured", provider)
            }
            OAuth2Error::NetworkError(msg) => write!(f, "network error: {}", msg),
        }
    }
}

impl std::error::Error for OAuth2Error {}

/// Configuration for a single OAuth2 provider.
#[derive(Debug, Clone)]
pub struct OAuth2ProviderConfig {
    /// Provider type
    pub provider: OAuth2Provider,
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Redirect URI for OAuth callback
    pub redirect_uri: String,
    /// Authorization endpoint URL
    pub authorization_endpoint: String,
    /// Token endpoint URL
    pub token_endpoint: String,
    /// User profile endpoint URL
    pub profile_endpoint: String,
    /// Scopes to request
    pub scopes: Vec<String>,
    /// Default role for new users
    pub default_role: UserRole,
}

impl OAuth2ProviderConfig {
    /// Create a GitHub OAuth2 configuration.
    pub fn github(client_id: &str, client_secret: &str, redirect_uri: &str) -> Self {
        Self {
            provider: OAuth2Provider::GitHub,
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            authorization_endpoint: "https://github.com/login/oauth/authorize".to_string(),
            token_endpoint: "https://github.com/login/oauth/access_token".to_string(),
            profile_endpoint: "https://api.github.com/user".to_string(),
            scopes: vec!["read:user".to_string(), "user:email".to_string()],
            default_role: UserRole::Viewer,
        }
    }

    /// Create a Google OAuth2 configuration.
    pub fn google(client_id: &str, client_secret: &str, redirect_uri: &str) -> Self {
        Self {
            provider: OAuth2Provider::Google,
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            authorization_endpoint: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_endpoint: "https://oauth2.googleapis.com/token".to_string(),
            profile_endpoint: "https://www.googleapis.com/oauth2/v2/userinfo".to_string(),
            scopes: vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
            default_role: UserRole::Viewer,
        }
    }

    /// Create a GitLab OAuth2 configuration.
    pub fn gitlab(client_id: &str, client_secret: &str, redirect_uri: &str) -> Self {
        Self::gitlab_with_host(client_id, client_secret, redirect_uri, "https://gitlab.com")
    }

    /// Create a GitLab OAuth2 configuration with a custom host (for self-hosted instances).
    pub fn gitlab_with_host(
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
        gitlab_host: &str,
    ) -> Self {
        let host = gitlab_host.trim_end_matches('/');
        Self {
            provider: OAuth2Provider::GitLab,
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            authorization_endpoint: format!("{}/oauth/authorize", host),
            token_endpoint: format!("{}/oauth/token", host),
            profile_endpoint: format!("{}/api/v4/user", host),
            scopes: vec!["read_user".to_string(), "email".to_string()],
            default_role: UserRole::Viewer,
        }
    }

    /// Set the default role for new users.
    pub fn with_default_role(mut self, role: UserRole) -> Self {
        self.default_role = role;
        self
    }

    /// Set custom scopes.
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }

    /// Validate the configuration is complete.
    pub fn validate(&self) -> Result<(), OAuth2Error> {
        if self.client_id.is_empty() {
            return Err(OAuth2Error::InvalidConfig("client_id is required".to_string()));
        }
        if self.client_secret.is_empty() {
            return Err(OAuth2Error::InvalidConfig(
                "client_secret is required".to_string(),
            ));
        }
        if self.redirect_uri.is_empty() {
            return Err(OAuth2Error::InvalidConfig(
                "redirect_uri is required".to_string(),
            ));
        }
        Ok(())
    }
}

/// Multi-provider OAuth2 configuration.
#[derive(Debug, Clone, Default)]
pub struct OAuth2Config {
    /// Configured providers
    pub providers: HashMap<OAuth2Provider, OAuth2ProviderConfig>,
}

impl OAuth2Config {
    /// Create a new empty configuration.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Add a provider configuration.
    pub fn with_provider(mut self, config: OAuth2ProviderConfig) -> Self {
        self.providers.insert(config.provider, config);
        self
    }

    /// Get a provider configuration.
    pub fn get_provider(&self, provider: OAuth2Provider) -> Option<&OAuth2ProviderConfig> {
        self.providers.get(&provider)
    }

    /// Check if a provider is configured.
    pub fn has_provider(&self, provider: OAuth2Provider) -> bool {
        self.providers.contains_key(&provider)
    }

    /// Get all configured providers.
    pub fn configured_providers(&self) -> Vec<OAuth2Provider> {
        self.providers.keys().copied().collect()
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let mut config = Self::new();

        // GitHub configuration
        if let (Ok(client_id), Ok(client_secret), Ok(redirect_uri)) = (
            std::env::var("GRALPH_OAUTH2_GITHUB_CLIENT_ID"),
            std::env::var("GRALPH_OAUTH2_GITHUB_CLIENT_SECRET"),
            std::env::var("GRALPH_OAUTH2_GITHUB_REDIRECT_URI"),
        ) {
            config = config.with_provider(OAuth2ProviderConfig::github(
                &client_id,
                &client_secret,
                &redirect_uri,
            ));
        }

        // Google configuration
        if let (Ok(client_id), Ok(client_secret), Ok(redirect_uri)) = (
            std::env::var("GRALPH_OAUTH2_GOOGLE_CLIENT_ID"),
            std::env::var("GRALPH_OAUTH2_GOOGLE_CLIENT_SECRET"),
            std::env::var("GRALPH_OAUTH2_GOOGLE_REDIRECT_URI"),
        ) {
            config = config.with_provider(OAuth2ProviderConfig::google(
                &client_id,
                &client_secret,
                &redirect_uri,
            ));
        }

        // GitLab configuration
        if let (Ok(client_id), Ok(client_secret), Ok(redirect_uri)) = (
            std::env::var("GRALPH_OAUTH2_GITLAB_CLIENT_ID"),
            std::env::var("GRALPH_OAUTH2_GITLAB_CLIENT_SECRET"),
            std::env::var("GRALPH_OAUTH2_GITLAB_REDIRECT_URI"),
        ) {
            let gitlab_host = std::env::var("GRALPH_OAUTH2_GITLAB_HOST")
                .unwrap_or_else(|_| "https://gitlab.com".to_string());
            config = config.with_provider(OAuth2ProviderConfig::gitlab_with_host(
                &client_id,
                &client_secret,
                &redirect_uri,
                &gitlab_host,
            ));
        }

        config
    }
}

/// OAuth2 authorization request tracking.
#[derive(Debug, Clone)]
pub struct OAuth2AuthRequest {
    /// State parameter for CSRF protection
    pub state: String,
    /// Provider for this request
    pub provider: OAuth2Provider,
    /// Redirect URL after successful auth
    pub redirect_url: Option<String>,
    /// User ID for account linking (if linking to existing account)
    pub link_to_user_id: Option<String>,
    /// Request creation timestamp
    pub created_at: u64,
    /// Request expiration timestamp
    pub expires_at: u64,
}

impl OAuth2AuthRequest {
    /// Create a new authorization request.
    pub fn new(provider: OAuth2Provider, redirect_url: Option<String>, validity_secs: u64) -> Self {
        let now = current_timestamp();
        Self {
            state: format!("oauth2_{}_{}", provider, Uuid::new_v4()),
            provider,
            redirect_url,
            link_to_user_id: None,
            created_at: now,
            expires_at: now + validity_secs,
        }
    }

    /// Create an authorization request for account linking.
    pub fn for_linking(
        provider: OAuth2Provider,
        user_id: &str,
        redirect_url: Option<String>,
        validity_secs: u64,
    ) -> Self {
        let mut request = Self::new(provider, redirect_url, validity_secs);
        request.link_to_user_id = Some(user_id.to_string());
        request
    }

    /// Check if the request has expired.
    pub fn is_expired(&self) -> bool {
        current_timestamp() > self.expires_at
    }
}

/// Token response from OAuth2 provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2TokenResponse {
    pub access_token: String,
    pub token_type: String,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

/// GitHub user profile response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUserProfile {
    pub id: u64,
    pub login: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

/// GitHub email response (for fetching primary email).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubEmail {
    pub email: String,
    pub primary: bool,
    pub verified: bool,
}

/// Google user profile response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleUserProfile {
    pub id: String,
    pub email: String,
    #[serde(default)]
    pub verified_email: Option<bool>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub given_name: Option<String>,
    #[serde(default)]
    pub family_name: Option<String>,
    #[serde(default)]
    pub picture: Option<String>,
}

/// GitLab user profile response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabUserProfile {
    pub id: u64,
    pub username: String,
    pub email: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub confirmed_at: Option<String>,
}

/// Unified OAuth2 user profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2UserProfile {
    /// Provider this profile came from
    pub provider: OAuth2Provider,
    /// Provider-specific user ID
    pub provider_user_id: String,
    /// User email
    pub email: String,
    /// Whether email is verified
    pub email_verified: bool,
    /// Display name
    pub name: Option<String>,
    /// Username (if available)
    pub username: Option<String>,
    /// Avatar URL
    pub avatar_url: Option<String>,
}

impl OAuth2UserProfile {
    /// Create from GitHub profile.
    pub fn from_github(profile: &GitHubUserProfile, email: Option<&str>) -> Self {
        Self {
            provider: OAuth2Provider::GitHub,
            provider_user_id: profile.id.to_string(),
            email: email
                .map(String::from)
                .or_else(|| profile.email.clone())
                .unwrap_or_default(),
            email_verified: email.is_some() || profile.email.is_some(),
            name: profile.name.clone(),
            username: Some(profile.login.clone()),
            avatar_url: profile.avatar_url.clone(),
        }
    }

    /// Create from Google profile.
    pub fn from_google(profile: &GoogleUserProfile) -> Self {
        Self {
            provider: OAuth2Provider::Google,
            provider_user_id: profile.id.clone(),
            email: profile.email.clone(),
            email_verified: profile.verified_email.unwrap_or(false),
            name: profile.name.clone(),
            username: None,
            avatar_url: profile.picture.clone(),
        }
    }

    /// Create from GitLab profile.
    pub fn from_gitlab(profile: &GitLabUserProfile) -> Self {
        Self {
            provider: OAuth2Provider::GitLab,
            provider_user_id: profile.id.to_string(),
            email: profile.email.clone(),
            email_verified: profile.confirmed_at.is_some(),
            name: profile.name.clone(),
            username: Some(profile.username.clone()),
            avatar_url: profile.avatar_url.clone(),
        }
    }
}

/// Linked account information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedAccount {
    /// Provider type
    pub provider: OAuth2Provider,
    /// Provider-specific user ID
    pub provider_user_id: String,
    /// Provider-specific email
    pub email: String,
    /// Username on the provider (if available)
    pub username: Option<String>,
    /// Linked at timestamp
    pub linked_at: u64,
}

/// Store for linked accounts.
#[derive(Debug, Clone)]
pub struct LinkedAccountStore {
    /// Maps user_id -> list of linked accounts
    accounts: Arc<RwLock<HashMap<String, Vec<LinkedAccount>>>>,
    /// Maps (provider, provider_user_id) -> user_id
    provider_index: Arc<RwLock<HashMap<(OAuth2Provider, String), String>>>,
}

impl LinkedAccountStore {
    /// Create a new linked account store.
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            provider_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Link an account to a user.
    pub fn link_account(
        &self,
        user_id: &str,
        profile: &OAuth2UserProfile,
    ) -> Result<LinkedAccount, OAuth2Error> {
        let linked_account = LinkedAccount {
            provider: profile.provider,
            provider_user_id: profile.provider_user_id.clone(),
            email: profile.email.clone(),
            username: profile.username.clone(),
            linked_at: current_timestamp(),
        };

        // Check if already linked to another user
        {
            let index = self.provider_index.read().unwrap();
            let key = (profile.provider, profile.provider_user_id.clone());
            if let Some(existing_user_id) = index.get(&key) {
                if existing_user_id != user_id {
                    return Err(OAuth2Error::LinkingFailed(
                        "account already linked to another user".to_string(),
                    ));
                }
            }
        }

        // Add to accounts
        {
            let mut accounts = self.accounts.write().unwrap();
            let user_accounts = accounts.entry(user_id.to_string()).or_default();

            // Check if already linked
            if user_accounts
                .iter()
                .any(|a| a.provider == profile.provider && a.provider_user_id == profile.provider_user_id)
            {
                return Err(OAuth2Error::LinkingFailed(
                    "account already linked".to_string(),
                ));
            }

            user_accounts.push(linked_account.clone());
        }

        // Update index
        {
            let mut index = self.provider_index.write().unwrap();
            index.insert(
                (profile.provider, profile.provider_user_id.clone()),
                user_id.to_string(),
            );
        }

        Ok(linked_account)
    }

    /// Unlink an account from a user.
    pub fn unlink_account(
        &self,
        user_id: &str,
        provider: OAuth2Provider,
    ) -> Result<(), OAuth2Error> {
        // Remove from accounts
        let removed_provider_user_id = {
            let mut accounts = self.accounts.write().unwrap();
            if let Some(user_accounts) = accounts.get_mut(user_id) {
                let original_len = user_accounts.len();
                let removed: Vec<_> = user_accounts
                    .iter()
                    .filter(|a| a.provider == provider)
                    .map(|a| a.provider_user_id.clone())
                    .collect();
                user_accounts.retain(|a| a.provider != provider);
                if user_accounts.len() == original_len {
                    return Err(OAuth2Error::LinkingFailed("account not linked".to_string()));
                }
                removed.into_iter().next()
            } else {
                return Err(OAuth2Error::LinkingFailed("user not found".to_string()));
            }
        };

        // Remove from index
        if let Some(provider_user_id) = removed_provider_user_id {
            let mut index = self.provider_index.write().unwrap();
            index.remove(&(provider, provider_user_id));
        }

        Ok(())
    }

    /// Get all linked accounts for a user.
    pub fn get_linked_accounts(&self, user_id: &str) -> Vec<LinkedAccount> {
        let accounts = self.accounts.read().unwrap();
        accounts.get(user_id).cloned().unwrap_or_default()
    }

    /// Find user ID by provider account.
    pub fn find_user_by_provider(
        &self,
        provider: OAuth2Provider,
        provider_user_id: &str,
    ) -> Option<String> {
        let index = self.provider_index.read().unwrap();
        index
            .get(&(provider, provider_user_id.to_string()))
            .cloned()
    }

    /// Check if a provider account is linked to any user.
    pub fn is_linked(&self, provider: OAuth2Provider, provider_user_id: &str) -> bool {
        self.find_user_by_provider(provider, provider_user_id)
            .is_some()
    }
}

impl Default for LinkedAccountStore {
    fn default() -> Self {
        Self::new()
    }
}

/// OAuth2 authentication result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2AuthResult {
    /// JWT token pair for the session
    pub tokens: TokenPair,
    /// OAuth2 user profile
    pub profile: OAuth2UserProfile,
    /// Redirect URL from original request
    pub redirect_url: Option<String>,
    /// Whether the user was newly provisioned
    pub is_new_user: bool,
    /// Whether this was an account linking operation
    pub is_linked: bool,
    /// OAuth2 access token (for provider API calls)
    pub oauth2_access_token: String,
    /// OAuth2 refresh token (if provided)
    pub oauth2_refresh_token: Option<String>,
}

/// OAuth2 client for handling provider authentication.
pub struct OAuth2Client {
    config: OAuth2Config,
    pending_requests: Arc<RwLock<HashMap<String, OAuth2AuthRequest>>>,
    request_validity_secs: u64,
}

impl OAuth2Client {
    /// Create a new OAuth2 client.
    pub fn new(config: OAuth2Config) -> Self {
        Self {
            config,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            request_validity_secs: 600, // 10 minutes
        }
    }

    /// Set request validity duration.
    pub fn with_request_validity(mut self, secs: u64) -> Self {
        self.request_validity_secs = secs;
        self
    }

    /// Get a provider configuration.
    pub fn get_provider_config(
        &self,
        provider: OAuth2Provider,
    ) -> Result<&OAuth2ProviderConfig, OAuth2Error> {
        self.config
            .get_provider(provider)
            .ok_or(OAuth2Error::ProviderNotConfigured(provider))
    }

    /// Get all configured providers.
    pub fn configured_providers(&self) -> Vec<OAuth2Provider> {
        self.config.configured_providers()
    }

    /// Create an authorization URL for a provider.
    pub fn create_authorization_url(
        &self,
        provider: OAuth2Provider,
        redirect_url: Option<String>,
    ) -> Result<(String, OAuth2AuthRequest), OAuth2Error> {
        let config = self.get_provider_config(provider)?;

        let request = OAuth2AuthRequest::new(provider, redirect_url, self.request_validity_secs);

        // Store the request
        {
            let mut requests = self.pending_requests.write().unwrap();
            // Clean up expired requests
            requests.retain(|_, req| !req.is_expired());
            requests.insert(request.state.clone(), request.clone());
        }

        // Build authorization URL
        let url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
            config.authorization_endpoint,
            urlencoding::encode(&config.client_id),
            urlencoding::encode(&config.redirect_uri),
            urlencoding::encode(&config.scopes.join(" ")),
            urlencoding::encode(&request.state),
        );

        Ok((url, request))
    }

    /// Create an authorization URL for account linking.
    pub fn create_linking_url(
        &self,
        provider: OAuth2Provider,
        user_id: &str,
        redirect_url: Option<String>,
    ) -> Result<(String, OAuth2AuthRequest), OAuth2Error> {
        let config = self.get_provider_config(provider)?;

        let request =
            OAuth2AuthRequest::for_linking(provider, user_id, redirect_url, self.request_validity_secs);

        // Store the request
        {
            let mut requests = self.pending_requests.write().unwrap();
            requests.retain(|_, req| !req.is_expired());
            requests.insert(request.state.clone(), request.clone());
        }

        // Build authorization URL
        let url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
            config.authorization_endpoint,
            urlencoding::encode(&config.client_id),
            urlencoding::encode(&config.redirect_uri),
            urlencoding::encode(&config.scopes.join(" ")),
            urlencoding::encode(&request.state),
        );

        Ok((url, request))
    }

    /// Validate an authorization callback.
    pub fn validate_callback(
        &self,
        state: &str,
        code: &str,
    ) -> Result<(OAuth2AuthRequest, String), OAuth2Error> {
        let request = {
            let mut requests = self.pending_requests.write().unwrap();
            let req = requests.remove(state).ok_or(OAuth2Error::InvalidState)?;

            if req.is_expired() {
                return Err(OAuth2Error::InvalidState);
            }

            req
        };

        Ok((request, code.to_string()))
    }

    /// Build token exchange request parameters.
    pub fn build_token_request(
        &self,
        provider: OAuth2Provider,
        code: &str,
    ) -> Result<OAuth2TokenRequestParams, OAuth2Error> {
        let config = self.get_provider_config(provider)?;

        Ok(OAuth2TokenRequestParams {
            endpoint: config.token_endpoint.clone(),
            grant_type: "authorization_code".to_string(),
            code: code.to_string(),
            redirect_uri: config.redirect_uri.clone(),
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
        })
    }

    /// Get the profile endpoint for a provider.
    pub fn profile_endpoint(&self, provider: OAuth2Provider) -> Result<String, OAuth2Error> {
        let config = self.get_provider_config(provider)?;
        Ok(config.profile_endpoint.clone())
    }
}

/// Parameters for OAuth2 token exchange request.
#[derive(Debug, Clone)]
pub struct OAuth2TokenRequestParams {
    pub endpoint: String,
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub client_secret: String,
}

impl OAuth2TokenRequestParams {
    /// Build form-encoded body for token request.
    pub fn to_form_body(&self) -> String {
        vec![
            format!("grant_type={}", urlencoding::encode(&self.grant_type)),
            format!("code={}", urlencoding::encode(&self.code)),
            format!("redirect_uri={}", urlencoding::encode(&self.redirect_uri)),
            format!("client_id={}", urlencoding::encode(&self.client_id)),
            format!("client_secret={}", urlencoding::encode(&self.client_secret)),
        ]
        .join("&")
    }
}

/// OAuth2 authentication service combining client with user and account stores.
pub struct OAuth2AuthService {
    client: OAuth2Client,
    user_store: UserStore,
    linked_accounts: LinkedAccountStore,
    jwt_config: crate::auth::JwtConfig,
}

impl OAuth2AuthService {
    /// Create a new OAuth2 authentication service.
    pub fn new(
        client: OAuth2Client,
        user_store: UserStore,
        jwt_config: crate::auth::JwtConfig,
    ) -> Self {
        Self {
            client,
            user_store,
            linked_accounts: LinkedAccountStore::new(),
            jwt_config,
        }
    }

    /// Create with an existing linked account store.
    pub fn with_linked_accounts(mut self, store: LinkedAccountStore) -> Self {
        self.linked_accounts = store;
        self
    }

    /// Get configured providers.
    pub fn configured_providers(&self) -> Vec<OAuth2Provider> {
        self.client.configured_providers()
    }

    /// Start OAuth2 authentication flow.
    pub fn start_auth(
        &self,
        provider: OAuth2Provider,
        redirect_url: Option<String>,
    ) -> Result<(String, OAuth2AuthRequest), OAuth2Error> {
        self.client.create_authorization_url(provider, redirect_url)
    }

    /// Start OAuth2 account linking flow.
    pub fn start_linking(
        &self,
        provider: OAuth2Provider,
        user_id: &str,
        redirect_url: Option<String>,
    ) -> Result<(String, OAuth2AuthRequest), OAuth2Error> {
        // Verify user exists
        if self.user_store.find_by_id(user_id).is_none() {
            return Err(OAuth2Error::LinkingFailed("user not found".to_string()));
        }

        self.client.create_linking_url(provider, user_id, redirect_url)
    }

    /// Validate an OAuth2 callback.
    pub fn validate_callback(
        &self,
        state: &str,
        code: &str,
    ) -> Result<(OAuth2AuthRequest, String), OAuth2Error> {
        self.client.validate_callback(state, code)
    }

    /// Build token exchange request parameters.
    pub fn build_token_request(
        &self,
        provider: OAuth2Provider,
        code: &str,
    ) -> Result<OAuth2TokenRequestParams, OAuth2Error> {
        self.client.build_token_request(provider, code)
    }

    /// Get profile endpoint for a provider.
    pub fn profile_endpoint(&self, provider: OAuth2Provider) -> Result<String, OAuth2Error> {
        self.client.profile_endpoint(provider)
    }

    /// Complete OAuth2 authentication after profile retrieval.
    pub fn complete_auth(
        &self,
        profile: OAuth2UserProfile,
        token_response: OAuth2TokenResponse,
        request: &OAuth2AuthRequest,
    ) -> Result<OAuth2AuthResult, OAuth2Error> {
        // Check if this is an account linking operation
        if let Some(ref user_id) = request.link_to_user_id {
            return self.complete_linking(user_id, profile, token_response, request);
        }

        // Check if provider account is already linked to a user
        if let Some(user_id) = self
            .linked_accounts
            .find_user_by_provider(profile.provider, &profile.provider_user_id)
        {
            // Login with existing linked account
            let user = self
                .user_store
                .find_by_id(&user_id)
                .ok_or_else(|| OAuth2Error::ProvisioningFailed("linked user not found".to_string()))?;

            let tokens = self
                .generate_tokens(&user)
                .map_err(|e| OAuth2Error::ProvisioningFailed(e.to_string()))?;

            return Ok(OAuth2AuthResult {
                tokens,
                profile,
                redirect_url: request.redirect_url.clone(),
                is_new_user: false,
                is_linked: true,
                oauth2_access_token: token_response.access_token,
                oauth2_refresh_token: token_response.refresh_token,
            });
        }

        // Check if user exists by email
        if let Some(existing_user) = self.user_store.find_by_email(&profile.email) {
            // Link the provider account to existing user
            self.linked_accounts
                .link_account(&existing_user.id, &profile)?;

            let tokens = self
                .generate_tokens(&existing_user)
                .map_err(|e| OAuth2Error::ProvisioningFailed(e.to_string()))?;

            return Ok(OAuth2AuthResult {
                tokens,
                profile,
                redirect_url: request.redirect_url.clone(),
                is_new_user: false,
                is_linked: true,
                oauth2_access_token: token_response.access_token,
                oauth2_refresh_token: token_response.refresh_token,
            });
        }

        // Provision new user
        let default_role = self
            .client
            .get_provider_config(profile.provider)
            .map(|c| c.default_role)
            .unwrap_or(UserRole::Viewer);

        let (user, is_new_user) = self.provision_user(&profile, default_role)?;

        // Link the provider account
        self.linked_accounts.link_account(&user.id, &profile)?;

        let tokens = self
            .generate_tokens(&user)
            .map_err(|e| OAuth2Error::ProvisioningFailed(e.to_string()))?;

        Ok(OAuth2AuthResult {
            tokens,
            profile,
            redirect_url: request.redirect_url.clone(),
            is_new_user,
            is_linked: true,
            oauth2_access_token: token_response.access_token,
            oauth2_refresh_token: token_response.refresh_token,
        })
    }

    /// Complete account linking operation.
    fn complete_linking(
        &self,
        user_id: &str,
        profile: OAuth2UserProfile,
        token_response: OAuth2TokenResponse,
        request: &OAuth2AuthRequest,
    ) -> Result<OAuth2AuthResult, OAuth2Error> {
        let user = self
            .user_store
            .find_by_id(user_id)
            .ok_or_else(|| OAuth2Error::LinkingFailed("user not found".to_string()))?;

        // Link the account
        self.linked_accounts.link_account(user_id, &profile)?;

        let tokens = self
            .generate_tokens(&user)
            .map_err(|e| OAuth2Error::ProvisioningFailed(e.to_string()))?;

        Ok(OAuth2AuthResult {
            tokens,
            profile,
            redirect_url: request.redirect_url.clone(),
            is_new_user: false,
            is_linked: true,
            oauth2_access_token: token_response.access_token,
            oauth2_refresh_token: token_response.refresh_token,
        })
    }

    /// Unlink a provider account from a user.
    pub fn unlink_account(
        &self,
        user_id: &str,
        provider: OAuth2Provider,
    ) -> Result<(), OAuth2Error> {
        self.linked_accounts.unlink_account(user_id, provider)
    }

    /// Get linked accounts for a user.
    pub fn get_linked_accounts(&self, user_id: &str) -> Vec<LinkedAccount> {
        self.linked_accounts.get_linked_accounts(user_id)
    }

    /// Provision a new user from OAuth2 profile.
    fn provision_user(
        &self,
        profile: &OAuth2UserProfile,
        role: UserRole,
    ) -> Result<(User, bool), OAuth2Error> {
        // Generate a random password (user will use OAuth2 for auth)
        let random_password = format!("OAuth2-{}-{}", Uuid::new_v4(), Uuid::new_v4());

        let user = self
            .user_store
            .register(&profile.email, &random_password, role)
            .map_err(|e| OAuth2Error::ProvisioningFailed(e.to_string()))?;

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
}

// Helper functions

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
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

    fn test_jwt_config() -> crate::auth::JwtConfig {
        crate::auth::JwtConfig::new("test-secret-key-for-testing")
            .with_expiry(60, 3600)
    }

    // OAuth2Provider tests

    #[test]
    fn oauth2_provider_display() {
        assert_eq!(format!("{}", OAuth2Provider::GitHub), "github");
        assert_eq!(format!("{}", OAuth2Provider::Google), "google");
        assert_eq!(format!("{}", OAuth2Provider::GitLab), "gitlab");
    }

    #[test]
    fn oauth2_provider_from_str() {
        assert_eq!("github".parse::<OAuth2Provider>().unwrap(), OAuth2Provider::GitHub);
        assert_eq!("Google".parse::<OAuth2Provider>().unwrap(), OAuth2Provider::Google);
        assert_eq!("GITLAB".parse::<OAuth2Provider>().unwrap(), OAuth2Provider::GitLab);
        assert!("unknown".parse::<OAuth2Provider>().is_err());
    }

    // OAuth2Error tests

    #[test]
    fn oauth2_error_display() {
        assert!(format!("{}", OAuth2Error::UnknownProvider("test".to_string())).contains("unknown"));
        assert!(format!("{}", OAuth2Error::InvalidConfig("test".to_string())).contains("configuration"));
        assert!(format!("{}", OAuth2Error::AuthorizationFailed("test".to_string())).contains("authorization"));
        assert!(format!("{}", OAuth2Error::TokenExchangeFailed("test".to_string())).contains("token exchange"));
        assert!(format!("{}", OAuth2Error::ProfileFailed("test".to_string())).contains("profile"));
        assert!(format!("{}", OAuth2Error::InvalidState).contains("state"));
        assert!(format!("{}", OAuth2Error::LinkingFailed("test".to_string())).contains("linking"));
        assert!(format!("{}", OAuth2Error::ProvisioningFailed("test".to_string())).contains("provisioning"));
        assert!(format!("{}", OAuth2Error::ProviderNotConfigured(OAuth2Provider::GitHub)).contains("github"));
        assert!(format!("{}", OAuth2Error::NetworkError("test".to_string())).contains("network"));
    }

    // OAuth2ProviderConfig tests

    #[test]
    fn github_config_defaults() {
        let config = OAuth2ProviderConfig::github("client-id", "client-secret", "https://app/callback");
        assert_eq!(config.provider, OAuth2Provider::GitHub);
        assert_eq!(config.client_id, "client-id");
        assert_eq!(config.client_secret, "client-secret");
        assert_eq!(config.redirect_uri, "https://app/callback");
        assert_eq!(config.authorization_endpoint, "https://github.com/login/oauth/authorize");
        assert_eq!(config.token_endpoint, "https://github.com/login/oauth/access_token");
        assert_eq!(config.profile_endpoint, "https://api.github.com/user");
        assert!(config.scopes.contains(&"read:user".to_string()));
        assert!(config.scopes.contains(&"user:email".to_string()));
    }

    #[test]
    fn google_config_defaults() {
        let config = OAuth2ProviderConfig::google("client-id", "client-secret", "https://app/callback");
        assert_eq!(config.provider, OAuth2Provider::Google);
        assert_eq!(config.authorization_endpoint, "https://accounts.google.com/o/oauth2/v2/auth");
        assert_eq!(config.token_endpoint, "https://oauth2.googleapis.com/token");
        assert_eq!(config.profile_endpoint, "https://www.googleapis.com/oauth2/v2/userinfo");
        assert!(config.scopes.contains(&"openid".to_string()));
        assert!(config.scopes.contains(&"email".to_string()));
        assert!(config.scopes.contains(&"profile".to_string()));
    }

    #[test]
    fn gitlab_config_defaults() {
        let config = OAuth2ProviderConfig::gitlab("client-id", "client-secret", "https://app/callback");
        assert_eq!(config.provider, OAuth2Provider::GitLab);
        assert_eq!(config.authorization_endpoint, "https://gitlab.com/oauth/authorize");
        assert_eq!(config.token_endpoint, "https://gitlab.com/oauth/token");
        assert_eq!(config.profile_endpoint, "https://gitlab.com/api/v4/user");
        assert!(config.scopes.contains(&"read_user".to_string()));
    }

    #[test]
    fn gitlab_config_custom_host() {
        let config = OAuth2ProviderConfig::gitlab_with_host(
            "client-id",
            "client-secret",
            "https://app/callback",
            "https://git.example.com/",
        );
        assert_eq!(config.authorization_endpoint, "https://git.example.com/oauth/authorize");
        assert_eq!(config.token_endpoint, "https://git.example.com/oauth/token");
        assert_eq!(config.profile_endpoint, "https://git.example.com/api/v4/user");
    }

    #[test]
    fn provider_config_validate_success() {
        let config = OAuth2ProviderConfig::github("id", "secret", "uri");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn provider_config_validate_missing_client_id() {
        let config = OAuth2ProviderConfig::github("", "secret", "uri");
        assert!(matches!(config.validate(), Err(OAuth2Error::InvalidConfig(_))));
    }

    #[test]
    fn provider_config_validate_missing_client_secret() {
        let config = OAuth2ProviderConfig::github("id", "", "uri");
        assert!(matches!(config.validate(), Err(OAuth2Error::InvalidConfig(_))));
    }

    #[test]
    fn provider_config_validate_missing_redirect_uri() {
        let config = OAuth2ProviderConfig::github("id", "secret", "");
        assert!(matches!(config.validate(), Err(OAuth2Error::InvalidConfig(_))));
    }

    #[test]
    fn provider_config_builder_methods() {
        let config = OAuth2ProviderConfig::github("id", "secret", "uri")
            .with_default_role(UserRole::Developer)
            .with_scopes(vec!["custom".to_string()]);
        assert_eq!(config.default_role, UserRole::Developer);
        assert_eq!(config.scopes, vec!["custom"]);
    }

    // OAuth2Config tests

    #[test]
    fn oauth2_config_new() {
        let config = OAuth2Config::new();
        assert!(config.providers.is_empty());
        assert!(config.configured_providers().is_empty());
    }

    #[test]
    fn oauth2_config_with_provider() {
        let config = OAuth2Config::new()
            .with_provider(OAuth2ProviderConfig::github("id", "secret", "uri"))
            .with_provider(OAuth2ProviderConfig::google("id", "secret", "uri"));

        assert!(config.has_provider(OAuth2Provider::GitHub));
        assert!(config.has_provider(OAuth2Provider::Google));
        assert!(!config.has_provider(OAuth2Provider::GitLab));
        assert_eq!(config.configured_providers().len(), 2);
    }

    #[test]
    fn oauth2_config_get_provider() {
        let config = OAuth2Config::new()
            .with_provider(OAuth2ProviderConfig::github("id", "secret", "uri"));

        assert!(config.get_provider(OAuth2Provider::GitHub).is_some());
        assert!(config.get_provider(OAuth2Provider::Google).is_none());
    }

    // OAuth2AuthRequest tests

    #[test]
    fn oauth2_auth_request_new() {
        let request = OAuth2AuthRequest::new(
            OAuth2Provider::GitHub,
            Some("/dashboard".to_string()),
            600,
        );
        assert!(request.state.starts_with("oauth2_github_"));
        assert_eq!(request.provider, OAuth2Provider::GitHub);
        assert_eq!(request.redirect_url, Some("/dashboard".to_string()));
        assert!(request.link_to_user_id.is_none());
        assert!(!request.is_expired());
    }

    #[test]
    fn oauth2_auth_request_for_linking() {
        let request = OAuth2AuthRequest::for_linking(
            OAuth2Provider::Google,
            "user-123",
            None,
            600,
        );
        assert_eq!(request.link_to_user_id, Some("user-123".to_string()));
    }

    #[test]
    fn oauth2_auth_request_expired() {
        let request = OAuth2AuthRequest {
            state: "state".to_string(),
            provider: OAuth2Provider::GitHub,
            redirect_url: None,
            link_to_user_id: None,
            created_at: 0,
            expires_at: 0,
        };
        assert!(request.is_expired());
    }

    // OAuth2UserProfile tests

    #[test]
    fn oauth2_user_profile_from_github() {
        let github_profile = GitHubUserProfile {
            id: 12345,
            login: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            avatar_url: Some("https://avatar.url".to_string()),
        };

        let profile = OAuth2UserProfile::from_github(&github_profile, None);
        assert_eq!(profile.provider, OAuth2Provider::GitHub);
        assert_eq!(profile.provider_user_id, "12345");
        assert_eq!(profile.email, "test@example.com");
        assert!(profile.email_verified);
        assert_eq!(profile.name, Some("Test User".to_string()));
        assert_eq!(profile.username, Some("testuser".to_string()));
    }

    #[test]
    fn oauth2_user_profile_from_github_with_email_override() {
        let github_profile = GitHubUserProfile {
            id: 12345,
            login: "testuser".to_string(),
            email: None,
            name: None,
            avatar_url: None,
        };

        let profile = OAuth2UserProfile::from_github(&github_profile, Some("primary@example.com"));
        assert_eq!(profile.email, "primary@example.com");
        assert!(profile.email_verified);
    }

    #[test]
    fn oauth2_user_profile_from_google() {
        let google_profile = GoogleUserProfile {
            id: "google-id".to_string(),
            email: "user@gmail.com".to_string(),
            verified_email: Some(true),
            name: Some("Google User".to_string()),
            given_name: Some("Google".to_string()),
            family_name: Some("User".to_string()),
            picture: Some("https://picture.url".to_string()),
        };

        let profile = OAuth2UserProfile::from_google(&google_profile);
        assert_eq!(profile.provider, OAuth2Provider::Google);
        assert_eq!(profile.provider_user_id, "google-id");
        assert_eq!(profile.email, "user@gmail.com");
        assert!(profile.email_verified);
        assert!(profile.username.is_none());
    }

    #[test]
    fn oauth2_user_profile_from_gitlab() {
        let gitlab_profile = GitLabUserProfile {
            id: 54321,
            username: "gitlabuser".to_string(),
            email: "user@gitlab.com".to_string(),
            name: Some("GitLab User".to_string()),
            avatar_url: Some("https://avatar.url".to_string()),
            confirmed_at: Some("2023-01-01T00:00:00Z".to_string()),
        };

        let profile = OAuth2UserProfile::from_gitlab(&gitlab_profile);
        assert_eq!(profile.provider, OAuth2Provider::GitLab);
        assert_eq!(profile.provider_user_id, "54321");
        assert_eq!(profile.email, "user@gitlab.com");
        assert!(profile.email_verified);
        assert_eq!(profile.username, Some("gitlabuser".to_string()));
    }

    #[test]
    fn oauth2_user_profile_from_gitlab_unconfirmed() {
        let gitlab_profile = GitLabUserProfile {
            id: 54321,
            username: "gitlabuser".to_string(),
            email: "user@gitlab.com".to_string(),
            name: None,
            avatar_url: None,
            confirmed_at: None,
        };

        let profile = OAuth2UserProfile::from_gitlab(&gitlab_profile);
        assert!(!profile.email_verified);
    }

    // LinkedAccountStore tests

    #[test]
    fn linked_account_store_link_account() {
        let store = LinkedAccountStore::new();
        let profile = OAuth2UserProfile {
            provider: OAuth2Provider::GitHub,
            provider_user_id: "12345".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            name: Some("Test".to_string()),
            username: Some("testuser".to_string()),
            avatar_url: None,
        };

        let linked = store.link_account("user-1", &profile).unwrap();
        assert_eq!(linked.provider, OAuth2Provider::GitHub);
        assert_eq!(linked.provider_user_id, "12345");
    }

    #[test]
    fn linked_account_store_prevent_duplicate_link() {
        let store = LinkedAccountStore::new();
        let profile = OAuth2UserProfile {
            provider: OAuth2Provider::GitHub,
            provider_user_id: "12345".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            name: None,
            username: None,
            avatar_url: None,
        };

        store.link_account("user-1", &profile).unwrap();
        let result = store.link_account("user-1", &profile);
        assert!(matches!(result, Err(OAuth2Error::LinkingFailed(_))));
    }

    #[test]
    fn linked_account_store_prevent_cross_user_link() {
        let store = LinkedAccountStore::new();
        let profile = OAuth2UserProfile {
            provider: OAuth2Provider::GitHub,
            provider_user_id: "12345".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            name: None,
            username: None,
            avatar_url: None,
        };

        store.link_account("user-1", &profile).unwrap();
        let result = store.link_account("user-2", &profile);
        assert!(matches!(result, Err(OAuth2Error::LinkingFailed(_))));
    }

    #[test]
    fn linked_account_store_find_user_by_provider() {
        let store = LinkedAccountStore::new();
        let profile = OAuth2UserProfile {
            provider: OAuth2Provider::GitHub,
            provider_user_id: "12345".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            name: None,
            username: None,
            avatar_url: None,
        };

        store.link_account("user-1", &profile).unwrap();

        assert_eq!(
            store.find_user_by_provider(OAuth2Provider::GitHub, "12345"),
            Some("user-1".to_string())
        );
        assert!(store.find_user_by_provider(OAuth2Provider::GitHub, "99999").is_none());
        assert!(store.find_user_by_provider(OAuth2Provider::Google, "12345").is_none());
    }

    #[test]
    fn linked_account_store_unlink_account() {
        let store = LinkedAccountStore::new();
        let profile = OAuth2UserProfile {
            provider: OAuth2Provider::GitHub,
            provider_user_id: "12345".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            name: None,
            username: None,
            avatar_url: None,
        };

        store.link_account("user-1", &profile).unwrap();
        assert!(store.is_linked(OAuth2Provider::GitHub, "12345"));

        store.unlink_account("user-1", OAuth2Provider::GitHub).unwrap();
        assert!(!store.is_linked(OAuth2Provider::GitHub, "12345"));
    }

    #[test]
    fn linked_account_store_unlink_nonexistent() {
        let store = LinkedAccountStore::new();
        let result = store.unlink_account("user-1", OAuth2Provider::GitHub);
        assert!(matches!(result, Err(OAuth2Error::LinkingFailed(_))));
    }

    #[test]
    fn linked_account_store_get_linked_accounts() {
        let store = LinkedAccountStore::new();

        let github_profile = OAuth2UserProfile {
            provider: OAuth2Provider::GitHub,
            provider_user_id: "github-123".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            name: None,
            username: None,
            avatar_url: None,
        };

        let google_profile = OAuth2UserProfile {
            provider: OAuth2Provider::Google,
            provider_user_id: "google-456".to_string(),
            email: "test@gmail.com".to_string(),
            email_verified: true,
            name: None,
            username: None,
            avatar_url: None,
        };

        store.link_account("user-1", &github_profile).unwrap();
        store.link_account("user-1", &google_profile).unwrap();

        let accounts = store.get_linked_accounts("user-1");
        assert_eq!(accounts.len(), 2);
    }

    // OAuth2TokenRequestParams tests

    #[test]
    fn token_request_params_to_form_body() {
        let params = OAuth2TokenRequestParams {
            endpoint: "https://example.com/token".to_string(),
            grant_type: "authorization_code".to_string(),
            code: "auth-code".to_string(),
            redirect_uri: "https://app/callback".to_string(),
            client_id: "client-id".to_string(),
            client_secret: "client-secret".to_string(),
        };

        let body = params.to_form_body();
        assert!(body.contains("grant_type=authorization_code"));
        assert!(body.contains("code=auth-code"));
        assert!(body.contains("redirect_uri="));
        assert!(body.contains("client_id=client-id"));
        assert!(body.contains("client_secret=client-secret"));
    }

    // OAuth2Client tests

    #[test]
    fn oauth2_client_configured_providers() {
        let config = OAuth2Config::new()
            .with_provider(OAuth2ProviderConfig::github("id", "secret", "uri"))
            .with_provider(OAuth2ProviderConfig::google("id", "secret", "uri"));

        let client = OAuth2Client::new(config);
        let providers = client.configured_providers();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&OAuth2Provider::GitHub));
        assert!(providers.contains(&OAuth2Provider::Google));
    }

    #[test]
    fn oauth2_client_get_provider_config() {
        let config = OAuth2Config::new()
            .with_provider(OAuth2ProviderConfig::github("id", "secret", "uri"));

        let client = OAuth2Client::new(config);
        assert!(client.get_provider_config(OAuth2Provider::GitHub).is_ok());
        assert!(matches!(
            client.get_provider_config(OAuth2Provider::Google),
            Err(OAuth2Error::ProviderNotConfigured(_))
        ));
    }

    #[test]
    fn oauth2_client_create_authorization_url() {
        let config = OAuth2Config::new()
            .with_provider(OAuth2ProviderConfig::github("client-id", "secret", "https://app/callback"));

        let client = OAuth2Client::new(config);
        let (url, request) = client
            .create_authorization_url(OAuth2Provider::GitHub, Some("/dashboard".to_string()))
            .unwrap();

        assert!(url.starts_with("https://github.com/login/oauth/authorize?"));
        assert!(url.contains("client_id=client-id"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("scope="));
        assert!(url.contains("state="));
        assert_eq!(request.provider, OAuth2Provider::GitHub);
        assert_eq!(request.redirect_url, Some("/dashboard".to_string()));
    }

    #[test]
    fn oauth2_client_create_linking_url() {
        let config = OAuth2Config::new()
            .with_provider(OAuth2ProviderConfig::github("client-id", "secret", "https://app/callback"));

        let client = OAuth2Client::new(config);
        let (url, request) = client
            .create_linking_url(OAuth2Provider::GitHub, "user-123", None)
            .unwrap();

        assert!(url.contains("github.com"));
        assert_eq!(request.link_to_user_id, Some("user-123".to_string()));
    }

    #[test]
    fn oauth2_client_validate_callback_invalid_state() {
        let config = OAuth2Config::new();
        let client = OAuth2Client::new(config);

        let result = client.validate_callback("unknown-state", "code");
        assert!(matches!(result, Err(OAuth2Error::InvalidState)));
    }

    #[test]
    fn oauth2_client_validate_callback_success() {
        let config = OAuth2Config::new()
            .with_provider(OAuth2ProviderConfig::github("id", "secret", "uri"));

        let client = OAuth2Client::new(config);
        let (_, request) = client
            .create_authorization_url(OAuth2Provider::GitHub, None)
            .unwrap();

        let (returned_request, code) = client.validate_callback(&request.state, "auth-code").unwrap();
        assert_eq!(returned_request.provider, OAuth2Provider::GitHub);
        assert_eq!(code, "auth-code");
    }

    #[test]
    fn oauth2_client_build_token_request() {
        let config = OAuth2Config::new()
            .with_provider(OAuth2ProviderConfig::github("client-id", "client-secret", "https://app/callback"));

        let client = OAuth2Client::new(config);
        let params = client
            .build_token_request(OAuth2Provider::GitHub, "auth-code")
            .unwrap();

        assert_eq!(params.endpoint, "https://github.com/login/oauth/access_token");
        assert_eq!(params.client_id, "client-id");
        assert_eq!(params.client_secret, "client-secret");
        assert_eq!(params.code, "auth-code");
    }

    // OAuth2AuthService tests

    #[test]
    fn oauth2_auth_service_start_auth() {
        let config = OAuth2Config::new()
            .with_provider(OAuth2ProviderConfig::github("id", "secret", "uri"));
        let client = OAuth2Client::new(config);
        let user_store = UserStore::new();
        let jwt_config = test_jwt_config();

        let service = OAuth2AuthService::new(client, user_store, jwt_config);
        let (url, request) = service
            .start_auth(OAuth2Provider::GitHub, Some("/dashboard".to_string()))
            .unwrap();

        assert!(url.contains("github.com"));
        assert_eq!(request.redirect_url, Some("/dashboard".to_string()));
    }

    #[test]
    fn oauth2_auth_service_start_linking_user_not_found() {
        let config = OAuth2Config::new()
            .with_provider(OAuth2ProviderConfig::github("id", "secret", "uri"));
        let client = OAuth2Client::new(config);
        let user_store = UserStore::new();
        let jwt_config = test_jwt_config();

        let service = OAuth2AuthService::new(client, user_store, jwt_config);
        let result = service.start_linking(OAuth2Provider::GitHub, "nonexistent-user", None);
        assert!(matches!(result, Err(OAuth2Error::LinkingFailed(_))));
    }

    #[test]
    fn oauth2_auth_service_complete_auth_new_user() {
        let config = OAuth2Config::new()
            .with_provider(OAuth2ProviderConfig::github("id", "secret", "uri"));
        let client = OAuth2Client::new(config);
        let user_store = UserStore::new();
        let jwt_config = test_jwt_config();

        let service = OAuth2AuthService::new(client, user_store, jwt_config);

        let profile = OAuth2UserProfile {
            provider: OAuth2Provider::GitHub,
            provider_user_id: "12345".to_string(),
            email: "newuser@example.com".to_string(),
            email_verified: true,
            name: Some("New User".to_string()),
            username: Some("newuser".to_string()),
            avatar_url: None,
        };

        let token_response = OAuth2TokenResponse {
            access_token: "access-token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            refresh_token: None,
            scope: None,
        };

        let request = OAuth2AuthRequest::new(OAuth2Provider::GitHub, None, 600);

        let result = service.complete_auth(profile, token_response, &request).unwrap();
        assert!(result.is_new_user);
        assert!(result.is_linked);
        assert!(!result.tokens.access_token.is_empty());
    }

    #[test]
    fn oauth2_auth_service_complete_auth_existing_user_by_email() {
        let config = OAuth2Config::new()
            .with_provider(OAuth2ProviderConfig::github("id", "secret", "uri"));
        let client = OAuth2Client::new(config);
        let user_store = UserStore::new();

        // Pre-register a user
        user_store
            .register("existing@example.com", "Password1", UserRole::Developer)
            .unwrap();

        let jwt_config = test_jwt_config();
        let service = OAuth2AuthService::new(client, user_store, jwt_config);

        let profile = OAuth2UserProfile {
            provider: OAuth2Provider::GitHub,
            provider_user_id: "12345".to_string(),
            email: "existing@example.com".to_string(),
            email_verified: true,
            name: None,
            username: None,
            avatar_url: None,
        };

        let token_response = OAuth2TokenResponse {
            access_token: "access-token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: None,
            refresh_token: None,
            scope: None,
        };

        let request = OAuth2AuthRequest::new(OAuth2Provider::GitHub, None, 600);

        let result = service.complete_auth(profile, token_response, &request).unwrap();
        assert!(!result.is_new_user);
        assert!(result.is_linked);
    }

    #[test]
    fn oauth2_auth_service_complete_auth_linked_account_login() {
        let config = OAuth2Config::new()
            .with_provider(OAuth2ProviderConfig::github("id", "secret", "uri"));
        let client = OAuth2Client::new(config);
        let user_store = UserStore::new();

        // Pre-register a user
        let user = user_store
            .register("user@example.com", "Password1", UserRole::Developer)
            .unwrap();

        let jwt_config = test_jwt_config();
        let linked_accounts = LinkedAccountStore::new();

        // Pre-link the GitHub account
        let profile = OAuth2UserProfile {
            provider: OAuth2Provider::GitHub,
            provider_user_id: "12345".to_string(),
            email: "github@example.com".to_string(),
            email_verified: true,
            name: None,
            username: None,
            avatar_url: None,
        };
        linked_accounts.link_account(&user.id, &profile).unwrap();

        let service = OAuth2AuthService::new(client, user_store, jwt_config)
            .with_linked_accounts(linked_accounts);

        let token_response = OAuth2TokenResponse {
            access_token: "access-token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: None,
            refresh_token: None,
            scope: None,
        };

        let request = OAuth2AuthRequest::new(OAuth2Provider::GitHub, None, 600);

        let result = service.complete_auth(profile, token_response, &request).unwrap();
        assert!(!result.is_new_user);
        assert!(result.is_linked);
    }

    #[test]
    fn oauth2_auth_service_get_linked_accounts() {
        let config = OAuth2Config::new();
        let client = OAuth2Client::new(config);
        let user_store = UserStore::new();
        let jwt_config = test_jwt_config();
        let linked_accounts = LinkedAccountStore::new();

        let profile = OAuth2UserProfile {
            provider: OAuth2Provider::GitHub,
            provider_user_id: "12345".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            name: None,
            username: None,
            avatar_url: None,
        };
        linked_accounts.link_account("user-1", &profile).unwrap();

        let service = OAuth2AuthService::new(client, user_store, jwt_config)
            .with_linked_accounts(linked_accounts);

        let accounts = service.get_linked_accounts("user-1");
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].provider, OAuth2Provider::GitHub);
    }

    #[test]
    fn oauth2_auth_service_unlink_account() {
        let config = OAuth2Config::new();
        let client = OAuth2Client::new(config);
        let user_store = UserStore::new();
        let jwt_config = test_jwt_config();
        let linked_accounts = LinkedAccountStore::new();

        let profile = OAuth2UserProfile {
            provider: OAuth2Provider::GitHub,
            provider_user_id: "12345".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            name: None,
            username: None,
            avatar_url: None,
        };
        linked_accounts.link_account("user-1", &profile).unwrap();

        let service = OAuth2AuthService::new(client, user_store, jwt_config)
            .with_linked_accounts(linked_accounts);

        assert!(service.unlink_account("user-1", OAuth2Provider::GitHub).is_ok());
        assert!(service.get_linked_accounts("user-1").is_empty());
    }

    // Profile response deserialization tests

    #[test]
    fn github_user_profile_deserialization() {
        let json = r#"{
            "id": 12345,
            "login": "testuser",
            "email": "test@example.com",
            "name": "Test User",
            "avatar_url": "https://avatar.url"
        }"#;

        let profile: GitHubUserProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.id, 12345);
        assert_eq!(profile.login, "testuser");
        assert_eq!(profile.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn github_user_profile_minimal() {
        let json = r#"{"id": 12345, "login": "testuser"}"#;
        let profile: GitHubUserProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.id, 12345);
        assert!(profile.email.is_none());
        assert!(profile.name.is_none());
    }

    #[test]
    fn github_email_deserialization() {
        let json = r#"{"email": "primary@example.com", "primary": true, "verified": true}"#;
        let email: GitHubEmail = serde_json::from_str(json).unwrap();
        assert_eq!(email.email, "primary@example.com");
        assert!(email.primary);
        assert!(email.verified);
    }

    #[test]
    fn google_user_profile_deserialization() {
        let json = r#"{
            "id": "google-id",
            "email": "user@gmail.com",
            "verified_email": true,
            "name": "Google User",
            "picture": "https://picture.url"
        }"#;

        let profile: GoogleUserProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.id, "google-id");
        assert_eq!(profile.email, "user@gmail.com");
        assert_eq!(profile.verified_email, Some(true));
    }

    #[test]
    fn gitlab_user_profile_deserialization() {
        let json = r#"{
            "id": 54321,
            "username": "gitlabuser",
            "email": "user@gitlab.com",
            "name": "GitLab User",
            "confirmed_at": "2023-01-01T00:00:00Z"
        }"#;

        let profile: GitLabUserProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.id, 54321);
        assert_eq!(profile.username, "gitlabuser");
        assert!(profile.confirmed_at.is_some());
    }

    #[test]
    fn oauth2_token_response_deserialization() {
        let json = r#"{
            "access_token": "access",
            "token_type": "Bearer",
            "expires_in": 3600,
            "refresh_token": "refresh"
        }"#;

        let response: OAuth2TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, "access");
        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, Some(3600));
        assert_eq!(response.refresh_token, Some("refresh".to_string()));
    }

    #[test]
    fn oauth2_token_response_minimal() {
        let json = r#"{"access_token": "access", "token_type": "Bearer"}"#;
        let response: OAuth2TokenResponse = serde_json::from_str(json).unwrap();
        assert!(response.expires_in.is_none());
        assert!(response.refresh_token.is_none());
    }
}

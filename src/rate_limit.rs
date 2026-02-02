//! Rate limiting middleware for protecting API endpoints.
//!
//! This module provides:
//! - Token bucket algorithm for flexible rate limiting
//! - Configurable limits per endpoint group (auth, api)
//! - 429 responses with Retry-After header
//! - Thread-safe rate limiting across concurrent requests

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Token bucket rate limiter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBucketConfig {
    /// Maximum number of tokens in the bucket (burst capacity)
    pub capacity: u32,
    /// Tokens added per second (refill rate)
    pub refill_rate: f64,
    /// Initial tokens in bucket (defaults to capacity)
    pub initial_tokens: Option<u32>,
}

impl Default for TokenBucketConfig {
    fn default() -> Self {
        Self {
            capacity: 100,
            refill_rate: 10.0,
            initial_tokens: None,
        }
    }
}

impl TokenBucketConfig {
    /// Create a new token bucket config with the given capacity and refill rate.
    pub fn new(capacity: u32, refill_rate: f64) -> Self {
        Self {
            capacity,
            refill_rate,
            initial_tokens: None,
        }
    }

    /// Set the initial number of tokens.
    pub fn with_initial_tokens(mut self, tokens: u32) -> Self {
        self.initial_tokens = Some(tokens);
        self
    }

    /// Create a strict config for auth endpoints (lower limits).
    pub fn auth_default() -> Self {
        Self {
            capacity: 10,
            refill_rate: 0.5, // 1 token per 2 seconds
            initial_tokens: None,
        }
    }

    /// Create a standard config for API endpoints.
    pub fn api_default() -> Self {
        Self {
            capacity: 100,
            refill_rate: 10.0, // 10 tokens per second
            initial_tokens: None,
        }
    }
}

/// Per-client token bucket state.
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    config: TokenBucketConfig,
}

impl TokenBucket {
    fn new(config: TokenBucketConfig) -> Self {
        let initial = config.initial_tokens.unwrap_or(config.capacity) as f64;
        Self {
            tokens: initial,
            last_refill: Instant::now(),
            config,
        }
    }

    /// Refill tokens based on elapsed time.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let added = elapsed * self.config.refill_rate;
        self.tokens = (self.tokens + added).min(self.config.capacity as f64);
        self.last_refill = now;
    }

    /// Try to consume a token. Returns Ok(()) if successful, Err with wait duration if rate limited.
    fn try_consume(&mut self) -> Result<(), Duration> {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            // Calculate time until next token is available
            let tokens_needed = 1.0 - self.tokens;
            let wait_secs = tokens_needed / self.config.refill_rate;
            Err(Duration::from_secs_f64(wait_secs))
        }
    }

    /// Get remaining tokens (floored to integer for display).
    fn remaining(&self) -> u32 {
        self.tokens.floor() as u32
    }

    /// Get time until bucket is fully refilled.
    fn time_to_full(&self) -> Duration {
        let tokens_needed = self.config.capacity as f64 - self.tokens;
        if tokens_needed <= 0.0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64(tokens_needed / self.config.refill_rate)
        }
    }
}

/// Endpoint group for rate limiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EndpointGroup {
    /// Authentication endpoints (login, register, refresh)
    Auth,
    /// General API endpoints
    Api,
    /// WebSocket connections
    WebSocket,
    /// Status/health endpoints (typically unlimited)
    Status,
}

impl std::fmt::Display for EndpointGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EndpointGroup::Auth => write!(f, "auth"),
            EndpointGroup::Api => write!(f, "api"),
            EndpointGroup::WebSocket => write!(f, "websocket"),
            EndpointGroup::Status => write!(f, "status"),
        }
    }
}

impl std::str::FromStr for EndpointGroup {
    type Err = RateLimitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auth" => Ok(EndpointGroup::Auth),
            "api" => Ok(EndpointGroup::Api),
            "websocket" | "ws" => Ok(EndpointGroup::WebSocket),
            "status" | "health" => Ok(EndpointGroup::Status),
            _ => Err(RateLimitError::InvalidEndpointGroup(s.to_string())),
        }
    }
}

/// Rate limiting middleware configuration.
#[derive(Debug, Clone)]
pub struct RateLimitMiddlewareConfig {
    /// Per-endpoint group configurations
    pub groups: HashMap<EndpointGroup, TokenBucketConfig>,
    /// Whether rate limiting is enabled
    pub enabled: bool,
    /// Whether to include rate limit headers in responses
    pub include_headers: bool,
    /// Trust X-Forwarded-For header for client IP
    pub trust_proxy: bool,
}

impl Default for RateLimitMiddlewareConfig {
    fn default() -> Self {
        let mut groups = HashMap::new();
        groups.insert(EndpointGroup::Auth, TokenBucketConfig::auth_default());
        groups.insert(EndpointGroup::Api, TokenBucketConfig::api_default());
        groups.insert(
            EndpointGroup::WebSocket,
            TokenBucketConfig::new(10, 1.0), // 10 connections, 1/sec refill
        );
        // Status endpoints are unlimited by default (not in groups)

        Self {
            groups,
            enabled: true,
            include_headers: true,
            trust_proxy: false,
        }
    }
}

impl RateLimitMiddlewareConfig {
    /// Create a new config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether rate limiting is enabled.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the config for an endpoint group.
    pub fn with_group(mut self, group: EndpointGroup, config: TokenBucketConfig) -> Self {
        self.groups.insert(group, config);
        self
    }

    /// Set whether to include rate limit headers.
    pub fn with_headers(mut self, include: bool) -> Self {
        self.include_headers = include;
        self
    }

    /// Set whether to trust X-Forwarded-For header.
    pub fn with_trust_proxy(mut self, trust: bool) -> Self {
        self.trust_proxy = trust;
        self
    }

    /// Load config from environment variables.
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // GRALPH_RATE_LIMIT_ENABLED
        if let Ok(enabled) = std::env::var("GRALPH_RATE_LIMIT_ENABLED") {
            config.enabled = enabled.to_lowercase() != "false" && enabled != "0";
        }

        // GRALPH_RATE_LIMIT_AUTH_CAPACITY
        if let Ok(capacity) = std::env::var("GRALPH_RATE_LIMIT_AUTH_CAPACITY") {
            if let Ok(cap) = capacity.parse::<u32>() {
                if let Some(auth_config) = config.groups.get_mut(&EndpointGroup::Auth) {
                    auth_config.capacity = cap;
                }
            }
        }

        // GRALPH_RATE_LIMIT_AUTH_REFILL_RATE
        if let Ok(rate) = std::env::var("GRALPH_RATE_LIMIT_AUTH_REFILL_RATE") {
            if let Ok(r) = rate.parse::<f64>() {
                if let Some(auth_config) = config.groups.get_mut(&EndpointGroup::Auth) {
                    auth_config.refill_rate = r;
                }
            }
        }

        // GRALPH_RATE_LIMIT_API_CAPACITY
        if let Ok(capacity) = std::env::var("GRALPH_RATE_LIMIT_API_CAPACITY") {
            if let Ok(cap) = capacity.parse::<u32>() {
                if let Some(api_config) = config.groups.get_mut(&EndpointGroup::Api) {
                    api_config.capacity = cap;
                }
            }
        }

        // GRALPH_RATE_LIMIT_API_REFILL_RATE
        if let Ok(rate) = std::env::var("GRALPH_RATE_LIMIT_API_REFILL_RATE") {
            if let Ok(r) = rate.parse::<f64>() {
                if let Some(api_config) = config.groups.get_mut(&EndpointGroup::Api) {
                    api_config.refill_rate = r;
                }
            }
        }

        // GRALPH_RATE_LIMIT_TRUST_PROXY
        if let Ok(trust) = std::env::var("GRALPH_RATE_LIMIT_TRUST_PROXY") {
            config.trust_proxy = trust.to_lowercase() == "true" || trust == "1";
        }

        config
    }
}

/// Rate limit error types.
#[derive(Debug, Clone)]
pub enum RateLimitError {
    /// Request rate limited, includes retry-after duration
    RateLimited {
        retry_after: Duration,
        endpoint_group: EndpointGroup,
    },
    /// Invalid endpoint group specified
    InvalidEndpointGroup(String),
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::RateLimited {
                retry_after,
                endpoint_group,
            } => {
                write!(
                    f,
                    "rate limited on {} endpoint, retry after {} seconds",
                    endpoint_group,
                    retry_after.as_secs()
                )
            }
            RateLimitError::InvalidEndpointGroup(group) => {
                write!(f, "invalid endpoint group: {}", group)
            }
        }
    }
}

impl std::error::Error for RateLimitError {}

/// Rate limit check result with metadata for response headers.
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Remaining requests in the current window
    pub remaining: u32,
    /// Total limit for this endpoint group
    pub limit: u32,
    /// Seconds until the limit resets (bucket is full)
    pub reset_after: u64,
    /// Seconds to wait before retrying (only set if rate limited)
    pub retry_after: Option<u64>,
    /// The endpoint group that was checked
    pub endpoint_group: EndpointGroup,
}

impl RateLimitResult {
    /// Convert to HTTP headers for rate limit information.
    pub fn to_headers(&self) -> Vec<(String, String)> {
        let mut headers = vec![
            ("X-RateLimit-Limit".to_string(), self.limit.to_string()),
            (
                "X-RateLimit-Remaining".to_string(),
                self.remaining.to_string(),
            ),
            (
                "X-RateLimit-Reset".to_string(),
                self.reset_after.to_string(),
            ),
        ];

        if let Some(retry) = self.retry_after {
            headers.push(("Retry-After".to_string(), retry.to_string()));
        }

        headers
    }
}

/// Key for identifying a rate limit bucket (client + endpoint group).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BucketKey {
    client_id: String,
    endpoint_group: EndpointGroup,
}

/// Rate limiting middleware.
#[derive(Debug, Clone)]
pub struct RateLimitMiddleware {
    config: RateLimitMiddlewareConfig,
    buckets: Arc<RwLock<HashMap<BucketKey, TokenBucket>>>,
}

impl RateLimitMiddleware {
    /// Create a new rate limit middleware with the given config.
    pub fn new(config: RateLimitMiddlewareConfig) -> Self {
        Self {
            config,
            buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new rate limit middleware with default config.
    pub fn with_defaults() -> Self {
        Self::new(RateLimitMiddlewareConfig::default())
    }

    /// Create a new rate limit middleware from environment variables.
    pub fn from_env() -> Self {
        Self::new(RateLimitMiddlewareConfig::from_env())
    }

    /// Check if a request is allowed for the given client and endpoint group.
    pub fn check(&self, client_id: &str, endpoint_group: EndpointGroup) -> RateLimitResult {
        // If rate limiting is disabled, always allow
        if !self.config.enabled {
            return RateLimitResult {
                allowed: true,
                remaining: u32::MAX,
                limit: u32::MAX,
                reset_after: 0,
                retry_after: None,
                endpoint_group,
            };
        }

        // Status endpoints are not rate limited
        if endpoint_group == EndpointGroup::Status {
            return RateLimitResult {
                allowed: true,
                remaining: u32::MAX,
                limit: u32::MAX,
                reset_after: 0,
                retry_after: None,
                endpoint_group,
            };
        }

        // Get the config for this endpoint group
        let bucket_config = self
            .config
            .groups
            .get(&endpoint_group)
            .cloned()
            .unwrap_or_default();

        let key = BucketKey {
            client_id: client_id.to_string(),
            endpoint_group,
        };

        let mut buckets = self.buckets.write().unwrap();
        let bucket = buckets
            .entry(key)
            .or_insert_with(|| TokenBucket::new(bucket_config.clone()));

        let limit = bucket.config.capacity;

        match bucket.try_consume() {
            Ok(()) => RateLimitResult {
                allowed: true,
                remaining: bucket.remaining(),
                limit,
                reset_after: bucket.time_to_full().as_secs(),
                retry_after: None,
                endpoint_group,
            },
            Err(wait_duration) => {
                let retry_secs = wait_duration.as_secs().max(1);
                RateLimitResult {
                    allowed: false,
                    remaining: 0,
                    limit,
                    reset_after: bucket.time_to_full().as_secs(),
                    retry_after: Some(retry_secs),
                    endpoint_group,
                }
            }
        }
    }

    /// Check rate limit and return error if limited.
    pub fn check_or_error(
        &self,
        client_id: &str,
        endpoint_group: EndpointGroup,
    ) -> Result<RateLimitResult, RateLimitError> {
        let result = self.check(client_id, endpoint_group);
        if result.allowed {
            Ok(result)
        } else {
            Err(RateLimitError::RateLimited {
                retry_after: Duration::from_secs(result.retry_after.unwrap_or(1)),
                endpoint_group,
            })
        }
    }

    /// Get the client identifier from request metadata.
    /// Uses X-Forwarded-For if trust_proxy is enabled, otherwise uses the direct IP.
    pub fn get_client_id(&self, remote_addr: &str, forwarded_for: Option<&str>) -> String {
        if self.config.trust_proxy {
            if let Some(xff) = forwarded_for {
                // Take the first IP in the X-Forwarded-For chain
                if let Some(first_ip) = xff.split(',').next() {
                    return first_ip.trim().to_string();
                }
            }
        }
        remote_addr.to_string()
    }

    /// Clear rate limit state for a specific client (e.g., after successful auth).
    pub fn clear_client(&self, client_id: &str) {
        let mut buckets = self.buckets.write().unwrap();
        buckets.retain(|key, _| key.client_id != client_id);
    }

    /// Clear rate limit state for a specific client and endpoint group.
    pub fn clear(&self, client_id: &str, endpoint_group: EndpointGroup) {
        let key = BucketKey {
            client_id: client_id.to_string(),
            endpoint_group,
        };
        let mut buckets = self.buckets.write().unwrap();
        buckets.remove(&key);
    }

    /// Get the remaining tokens for a client and endpoint group.
    pub fn remaining(&self, client_id: &str, endpoint_group: EndpointGroup) -> u32 {
        if !self.config.enabled || endpoint_group == EndpointGroup::Status {
            return u32::MAX;
        }

        let key = BucketKey {
            client_id: client_id.to_string(),
            endpoint_group,
        };

        let buckets = self.buckets.read().unwrap();
        match buckets.get(&key) {
            Some(bucket) => bucket.remaining(),
            None => {
                // Return capacity if no bucket exists yet
                self.config
                    .groups
                    .get(&endpoint_group)
                    .map(|c| c.capacity)
                    .unwrap_or(100)
            }
        }
    }

    /// Check if rate limiting is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if headers should be included in responses.
    pub fn include_headers(&self) -> bool {
        self.config.include_headers
    }

    /// Cleanup stale buckets that haven't been used recently.
    /// Call this periodically to prevent memory growth.
    pub fn cleanup(&self, max_age: Duration) {
        let now = Instant::now();
        let mut buckets = self.buckets.write().unwrap();
        buckets.retain(|_, bucket| {
            // Keep buckets that have been used within max_age
            now.duration_since(bucket.last_refill) < max_age
        });
    }
}

impl Default for RateLimitMiddleware {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Determine the endpoint group for a given path.
pub fn endpoint_group_for_path(path: &str) -> EndpointGroup {
    let path_lower = path.to_lowercase();

    // Auth endpoints
    if path_lower.starts_with("/auth/")
        || path_lower.starts_with("/saml/")
        || path_lower.starts_with("/oauth2/")
        || path_lower.starts_with("/oidc/")
    {
        return EndpointGroup::Auth;
    }

    // Status/health endpoints
    if path_lower == "/health" || path_lower == "/" || path_lower.starts_with("/health/") {
        return EndpointGroup::Status;
    }

    // WebSocket
    if path_lower == "/ws" || path_lower.starts_with("/ws/") {
        return EndpointGroup::WebSocket;
    }

    // Default to API
    EndpointGroup::Api
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn token_bucket_config_default() {
        let config = TokenBucketConfig::default();
        assert_eq!(config.capacity, 100);
        assert_eq!(config.refill_rate, 10.0);
        assert!(config.initial_tokens.is_none());
    }

    #[test]
    fn token_bucket_config_auth_default() {
        let config = TokenBucketConfig::auth_default();
        assert_eq!(config.capacity, 10);
        assert_eq!(config.refill_rate, 0.5);
    }

    #[test]
    fn token_bucket_config_api_default() {
        let config = TokenBucketConfig::api_default();
        assert_eq!(config.capacity, 100);
        assert_eq!(config.refill_rate, 10.0);
    }

    #[test]
    fn token_bucket_config_with_initial_tokens() {
        let config = TokenBucketConfig::new(50, 5.0).with_initial_tokens(25);
        assert_eq!(config.capacity, 50);
        assert_eq!(config.refill_rate, 5.0);
        assert_eq!(config.initial_tokens, Some(25));
    }

    #[test]
    fn token_bucket_allows_requests_under_limit() {
        let config = TokenBucketConfig::new(5, 1.0);
        let mut bucket = TokenBucket::new(config);

        // Should allow 5 requests
        for _ in 0..5 {
            assert!(bucket.try_consume().is_ok());
        }
    }

    #[test]
    fn token_bucket_blocks_requests_over_limit() {
        let config = TokenBucketConfig::new(3, 1.0);
        let mut bucket = TokenBucket::new(config);

        // Use up all tokens
        for _ in 0..3 {
            assert!(bucket.try_consume().is_ok());
        }

        // Next request should be rate limited
        let result = bucket.try_consume();
        assert!(result.is_err());
        let wait_duration = result.unwrap_err();
        assert!(wait_duration.as_secs_f64() > 0.0);
    }

    #[test]
    fn token_bucket_refills_over_time() {
        let config = TokenBucketConfig::new(2, 100.0); // Fast refill for testing
        let mut bucket = TokenBucket::new(config);

        // Use all tokens
        assert!(bucket.try_consume().is_ok());
        assert!(bucket.try_consume().is_ok());
        assert!(bucket.try_consume().is_err());

        // Wait a bit for refill
        thread::sleep(Duration::from_millis(20));

        // Should have some tokens now
        assert!(bucket.try_consume().is_ok());
    }

    #[test]
    fn token_bucket_remaining_count() {
        let config = TokenBucketConfig::new(5, 1.0);
        let mut bucket = TokenBucket::new(config);

        assert_eq!(bucket.remaining(), 5);
        bucket.try_consume().unwrap();
        assert_eq!(bucket.remaining(), 4);
        bucket.try_consume().unwrap();
        assert_eq!(bucket.remaining(), 3);
    }

    #[test]
    fn endpoint_group_display() {
        assert_eq!(EndpointGroup::Auth.to_string(), "auth");
        assert_eq!(EndpointGroup::Api.to_string(), "api");
        assert_eq!(EndpointGroup::WebSocket.to_string(), "websocket");
        assert_eq!(EndpointGroup::Status.to_string(), "status");
    }

    #[test]
    fn endpoint_group_from_str() {
        assert_eq!("auth".parse::<EndpointGroup>().unwrap(), EndpointGroup::Auth);
        assert_eq!("api".parse::<EndpointGroup>().unwrap(), EndpointGroup::Api);
        assert_eq!(
            "websocket".parse::<EndpointGroup>().unwrap(),
            EndpointGroup::WebSocket
        );
        assert_eq!("ws".parse::<EndpointGroup>().unwrap(), EndpointGroup::WebSocket);
        assert_eq!(
            "status".parse::<EndpointGroup>().unwrap(),
            EndpointGroup::Status
        );
        assert_eq!(
            "health".parse::<EndpointGroup>().unwrap(),
            EndpointGroup::Status
        );
        assert!("invalid".parse::<EndpointGroup>().is_err());
    }

    #[test]
    fn rate_limit_middleware_config_default() {
        let config = RateLimitMiddlewareConfig::default();
        assert!(config.enabled);
        assert!(config.include_headers);
        assert!(!config.trust_proxy);
        assert!(config.groups.contains_key(&EndpointGroup::Auth));
        assert!(config.groups.contains_key(&EndpointGroup::Api));
    }

    #[test]
    fn rate_limit_middleware_config_builders() {
        let config = RateLimitMiddlewareConfig::new()
            .with_enabled(false)
            .with_headers(false)
            .with_trust_proxy(true)
            .with_group(EndpointGroup::Auth, TokenBucketConfig::new(5, 0.1));

        assert!(!config.enabled);
        assert!(!config.include_headers);
        assert!(config.trust_proxy);
        assert_eq!(config.groups.get(&EndpointGroup::Auth).unwrap().capacity, 5);
    }

    #[test]
    fn rate_limit_middleware_allows_when_disabled() {
        let config = RateLimitMiddlewareConfig::new().with_enabled(false);
        let middleware = RateLimitMiddleware::new(config);

        let result = middleware.check("127.0.0.1", EndpointGroup::Auth);
        assert!(result.allowed);
        assert_eq!(result.remaining, u32::MAX);
    }

    #[test]
    fn rate_limit_middleware_always_allows_status() {
        let middleware = RateLimitMiddleware::with_defaults();

        // Status endpoints are never rate limited
        for _ in 0..1000 {
            let result = middleware.check("127.0.0.1", EndpointGroup::Status);
            assert!(result.allowed);
        }
    }

    #[test]
    fn rate_limit_middleware_limits_auth_endpoints() {
        let config = RateLimitMiddlewareConfig::new()
            .with_group(EndpointGroup::Auth, TokenBucketConfig::new(3, 0.1));
        let middleware = RateLimitMiddleware::new(config);

        // First 3 requests should succeed
        for _ in 0..3 {
            let result = middleware.check("127.0.0.1", EndpointGroup::Auth);
            assert!(result.allowed);
        }

        // Next request should be rate limited
        let result = middleware.check("127.0.0.1", EndpointGroup::Auth);
        assert!(!result.allowed);
        assert!(result.retry_after.is_some());
    }

    #[test]
    fn rate_limit_middleware_tracks_clients_separately() {
        let config = RateLimitMiddlewareConfig::new()
            .with_group(EndpointGroup::Api, TokenBucketConfig::new(2, 0.1));
        let middleware = RateLimitMiddleware::new(config);

        // Use up client1's tokens
        middleware.check("client1", EndpointGroup::Api);
        middleware.check("client1", EndpointGroup::Api);
        let result1 = middleware.check("client1", EndpointGroup::Api);
        assert!(!result1.allowed);

        // client2 should still have tokens
        let result2 = middleware.check("client2", EndpointGroup::Api);
        assert!(result2.allowed);
    }

    #[test]
    fn rate_limit_middleware_tracks_groups_separately() {
        let config = RateLimitMiddlewareConfig::new()
            .with_group(EndpointGroup::Auth, TokenBucketConfig::new(2, 0.1))
            .with_group(EndpointGroup::Api, TokenBucketConfig::new(2, 0.1));
        let middleware = RateLimitMiddleware::new(config);

        // Use up Auth tokens
        middleware.check("client", EndpointGroup::Auth);
        middleware.check("client", EndpointGroup::Auth);
        let auth_result = middleware.check("client", EndpointGroup::Auth);
        assert!(!auth_result.allowed);

        // API should still have tokens
        let api_result = middleware.check("client", EndpointGroup::Api);
        assert!(api_result.allowed);
    }

    #[test]
    fn rate_limit_middleware_check_or_error() {
        let config = RateLimitMiddlewareConfig::new()
            .with_group(EndpointGroup::Auth, TokenBucketConfig::new(1, 0.1));
        let middleware = RateLimitMiddleware::new(config);

        assert!(middleware
            .check_or_error("client", EndpointGroup::Auth)
            .is_ok());
        assert!(middleware
            .check_or_error("client", EndpointGroup::Auth)
            .is_err());
    }

    #[test]
    fn rate_limit_middleware_get_client_id_direct() {
        let middleware = RateLimitMiddleware::with_defaults();

        let id = middleware.get_client_id("192.168.1.1", None);
        assert_eq!(id, "192.168.1.1");

        // Without trust_proxy, ignores forwarded header
        let id = middleware.get_client_id("192.168.1.1", Some("10.0.0.1"));
        assert_eq!(id, "192.168.1.1");
    }

    #[test]
    fn rate_limit_middleware_get_client_id_with_proxy() {
        let config = RateLimitMiddlewareConfig::new().with_trust_proxy(true);
        let middleware = RateLimitMiddleware::new(config);

        // With trust_proxy, uses X-Forwarded-For
        let id = middleware.get_client_id("192.168.1.1", Some("10.0.0.1"));
        assert_eq!(id, "10.0.0.1");

        // First IP in chain
        let id = middleware.get_client_id("192.168.1.1", Some("10.0.0.1, 172.16.0.1"));
        assert_eq!(id, "10.0.0.1");

        // Falls back to direct IP if no header
        let id = middleware.get_client_id("192.168.1.1", None);
        assert_eq!(id, "192.168.1.1");
    }

    #[test]
    fn rate_limit_middleware_clear_client() {
        let config = RateLimitMiddlewareConfig::new()
            .with_group(EndpointGroup::Auth, TokenBucketConfig::new(1, 0.1));
        let middleware = RateLimitMiddleware::new(config);

        middleware.check("client", EndpointGroup::Auth);
        let result = middleware.check("client", EndpointGroup::Auth);
        assert!(!result.allowed);

        // Clear the client
        middleware.clear_client("client");

        // Should have full tokens again
        let result = middleware.check("client", EndpointGroup::Auth);
        assert!(result.allowed);
    }

    #[test]
    fn rate_limit_middleware_clear_specific() {
        let config = RateLimitMiddlewareConfig::new()
            .with_group(EndpointGroup::Auth, TokenBucketConfig::new(1, 0.1))
            .with_group(EndpointGroup::Api, TokenBucketConfig::new(1, 0.1));
        let middleware = RateLimitMiddleware::new(config);

        middleware.check("client", EndpointGroup::Auth);
        middleware.check("client", EndpointGroup::Api);

        // Both should be rate limited
        assert!(!middleware.check("client", EndpointGroup::Auth).allowed);
        assert!(!middleware.check("client", EndpointGroup::Api).allowed);

        // Clear only Auth
        middleware.clear("client", EndpointGroup::Auth);

        // Auth should work, API still limited
        assert!(middleware.check("client", EndpointGroup::Auth).allowed);
        assert!(!middleware.check("client", EndpointGroup::Api).allowed);
    }

    #[test]
    fn rate_limit_middleware_remaining() {
        let config = RateLimitMiddlewareConfig::new()
            .with_group(EndpointGroup::Api, TokenBucketConfig::new(5, 0.1));
        let middleware = RateLimitMiddleware::new(config);

        assert_eq!(middleware.remaining("client", EndpointGroup::Api), 5);

        middleware.check("client", EndpointGroup::Api);
        assert_eq!(middleware.remaining("client", EndpointGroup::Api), 4);
    }

    #[test]
    fn rate_limit_result_to_headers() {
        let result = RateLimitResult {
            allowed: false,
            remaining: 0,
            limit: 100,
            reset_after: 60,
            retry_after: Some(5),
            endpoint_group: EndpointGroup::Api,
        };

        let headers = result.to_headers();
        assert!(headers.iter().any(|(k, v)| k == "X-RateLimit-Limit" && v == "100"));
        assert!(headers.iter().any(|(k, v)| k == "X-RateLimit-Remaining" && v == "0"));
        assert!(headers.iter().any(|(k, v)| k == "X-RateLimit-Reset" && v == "60"));
        assert!(headers.iter().any(|(k, v)| k == "Retry-After" && v == "5"));
    }

    #[test]
    fn rate_limit_result_to_headers_no_retry() {
        let result = RateLimitResult {
            allowed: true,
            remaining: 50,
            limit: 100,
            reset_after: 30,
            retry_after: None,
            endpoint_group: EndpointGroup::Api,
        };

        let headers = result.to_headers();
        assert!(!headers.iter().any(|(k, _)| k == "Retry-After"));
    }

    #[test]
    fn endpoint_group_for_path_auth() {
        assert_eq!(endpoint_group_for_path("/auth/login"), EndpointGroup::Auth);
        assert_eq!(endpoint_group_for_path("/auth/register"), EndpointGroup::Auth);
        assert_eq!(endpoint_group_for_path("/AUTH/REFRESH"), EndpointGroup::Auth);
        assert_eq!(endpoint_group_for_path("/saml/acs"), EndpointGroup::Auth);
        assert_eq!(endpoint_group_for_path("/oauth2/callback"), EndpointGroup::Auth);
        assert_eq!(endpoint_group_for_path("/oidc/callback"), EndpointGroup::Auth);
    }

    #[test]
    fn endpoint_group_for_path_status() {
        assert_eq!(endpoint_group_for_path("/health"), EndpointGroup::Status);
        assert_eq!(endpoint_group_for_path("/"), EndpointGroup::Status);
        assert_eq!(endpoint_group_for_path("/health/ready"), EndpointGroup::Status);
    }

    #[test]
    fn endpoint_group_for_path_websocket() {
        assert_eq!(endpoint_group_for_path("/ws"), EndpointGroup::WebSocket);
        assert_eq!(endpoint_group_for_path("/ws/connect"), EndpointGroup::WebSocket);
    }

    #[test]
    fn endpoint_group_for_path_api() {
        assert_eq!(endpoint_group_for_path("/status"), EndpointGroup::Api);
        assert_eq!(endpoint_group_for_path("/tasks/session"), EndpointGroup::Api);
        assert_eq!(endpoint_group_for_path("/logs/demo"), EndpointGroup::Api);
        assert_eq!(endpoint_group_for_path("/orchestration"), EndpointGroup::Api);
    }

    #[test]
    fn rate_limit_error_display() {
        let error = RateLimitError::RateLimited {
            retry_after: Duration::from_secs(30),
            endpoint_group: EndpointGroup::Auth,
        };
        assert!(error.to_string().contains("rate limited"));
        assert!(error.to_string().contains("auth"));
        assert!(error.to_string().contains("30"));

        let error = RateLimitError::InvalidEndpointGroup("foo".to_string());
        assert!(error.to_string().contains("invalid endpoint group"));
        assert!(error.to_string().contains("foo"));
    }

    #[test]
    fn rate_limit_middleware_cleanup() {
        let config = RateLimitMiddlewareConfig::new()
            .with_group(EndpointGroup::Api, TokenBucketConfig::new(5, 0.1));
        let middleware = RateLimitMiddleware::new(config);

        middleware.check("client1", EndpointGroup::Api);
        middleware.check("client2", EndpointGroup::Api);

        // Both clients have buckets
        assert_eq!(middleware.remaining("client1", EndpointGroup::Api), 4);
        assert_eq!(middleware.remaining("client2", EndpointGroup::Api), 4);

        // Cleanup with short max_age won't remove recent buckets
        middleware.cleanup(Duration::from_secs(1));
        assert_eq!(middleware.remaining("client1", EndpointGroup::Api), 4);

        // Cleanup with zero duration removes all
        middleware.cleanup(Duration::ZERO);
        // After cleanup, clients get fresh buckets
        assert_eq!(middleware.remaining("client1", EndpointGroup::Api), 5);
    }

    #[test]
    fn token_bucket_time_to_full() {
        let config = TokenBucketConfig::new(10, 2.0); // 2 tokens/sec
        let mut bucket = TokenBucket::new(config);

        // Full bucket has zero time to full
        assert_eq!(bucket.time_to_full().as_secs(), 0);

        // Use some tokens
        bucket.try_consume().unwrap();
        bucket.try_consume().unwrap();
        bucket.try_consume().unwrap();

        // Should take ~1.5 seconds to refill 3 tokens at 2/sec
        let time = bucket.time_to_full();
        assert!(time.as_secs_f64() > 1.0);
        assert!(time.as_secs_f64() < 2.0);
    }

    #[test]
    fn rate_limit_middleware_is_enabled() {
        let enabled = RateLimitMiddleware::with_defaults();
        assert!(enabled.is_enabled());

        let disabled = RateLimitMiddleware::new(
            RateLimitMiddlewareConfig::new().with_enabled(false),
        );
        assert!(!disabled.is_enabled());
    }

    #[test]
    fn rate_limit_middleware_include_headers() {
        let with_headers = RateLimitMiddleware::with_defaults();
        assert!(with_headers.include_headers());

        let without_headers = RateLimitMiddleware::new(
            RateLimitMiddlewareConfig::new().with_headers(false),
        );
        assert!(!without_headers.include_headers());
    }
}

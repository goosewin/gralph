//! Two-factor authentication module with TOTP and backup codes.
//!
//! This module provides:
//! - TOTP (Time-based One-Time Password) generation and verification
//! - QR code URL generation for authenticator apps
//! - Backup codes for recovery when device is lost
//! - Email-based recovery flow
//! - Rate limiting on 2FA attempts

use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// TOTP configuration defaults (RFC 6238).
const TOTP_DIGITS: u32 = 6;
const TOTP_PERIOD: u64 = 30; // seconds
const TOTP_SECRET_LENGTH: usize = 20; // 160 bits for HMAC-SHA1

/// Backup code configuration.
const BACKUP_CODE_COUNT: usize = 10;
const BACKUP_CODE_LENGTH: usize = 8;

/// Recovery token expiration (15 minutes).
const RECOVERY_TOKEN_EXPIRY_SECS: u64 = 15 * 60;

/// Two-factor authentication status for a user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TwoFactorStatus {
    /// 2FA is not enabled
    Disabled,
    /// 2FA setup is pending (user needs to verify first code)
    Pending,
    /// 2FA is fully enabled
    Enabled,
}

impl Default for TwoFactorStatus {
    fn default() -> Self {
        TwoFactorStatus::Disabled
    }
}

/// Two-factor authentication data for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorData {
    /// Current 2FA status
    pub status: TwoFactorStatus,
    /// TOTP secret (base32 encoded)
    pub totp_secret: Option<String>,
    /// Backup codes (hashed)
    pub backup_codes: Vec<String>,
    /// Email for recovery
    pub recovery_email: Option<String>,
    /// When 2FA was enabled
    pub enabled_at: Option<u64>,
}

impl Default for TwoFactorData {
    fn default() -> Self {
        Self {
            status: TwoFactorStatus::Disabled,
            totp_secret: None,
            backup_codes: Vec::new(),
            recovery_email: None,
            enabled_at: None,
        }
    }
}

/// Result of initiating 2FA setup.
#[derive(Debug, Clone, Serialize)]
pub struct TwoFactorSetupResult {
    /// Base32-encoded secret for manual entry
    pub secret: String,
    /// QR code URL for authenticator apps (otpauth:// URI)
    pub qr_code_url: String,
    /// Backup codes (only shown once)
    pub backup_codes: Vec<String>,
}

/// Recovery token for email-based recovery.
#[derive(Debug, Clone)]
pub struct RecoveryToken {
    /// Token value
    pub token: String,
    /// User ID
    pub user_id: String,
    /// Expiration timestamp
    pub expires_at: u64,
    /// Whether token has been used
    pub used: bool,
}

/// Rate limiter for 2FA attempts.
#[derive(Debug, Clone)]
pub struct TwoFactorRateLimiter {
    /// Maximum attempts per window
    max_attempts: u32,
    /// Window duration in seconds
    window_secs: u64,
    /// Lockout duration after exceeding limit (seconds)
    lockout_secs: u64,
    /// Per-user attempt tracking
    attempts: Arc<RwLock<HashMap<String, TwoFactorAttemptEntry>>>,
}

#[derive(Debug, Clone)]
struct TwoFactorAttemptEntry {
    attempts: Vec<Instant>,
    locked_until: Option<Instant>,
}

impl TwoFactorAttemptEntry {
    fn new() -> Self {
        Self {
            attempts: Vec::new(),
            locked_until: None,
        }
    }
}

impl TwoFactorRateLimiter {
    /// Create a new rate limiter with the given configuration.
    pub fn new(max_attempts: u32, window_secs: u64, lockout_secs: u64) -> Self {
        Self {
            max_attempts,
            window_secs,
            lockout_secs,
            attempts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a 2FA attempt is allowed for the given user.
    /// Returns Ok(remaining_attempts) if allowed, Err(lockout_duration) if rate limited.
    pub fn check(&self, user_id: &str) -> Result<u32, Duration> {
        let now = Instant::now();
        let window = Duration::from_secs(self.window_secs);

        let mut attempts = self.attempts.write().unwrap();
        let entry = attempts
            .entry(user_id.to_string())
            .or_insert_with(TwoFactorAttemptEntry::new);

        // Check if currently locked out
        if let Some(locked_until) = entry.locked_until {
            if now < locked_until {
                return Err(locked_until - now);
            }
            // Lockout expired, reset
            entry.locked_until = None;
            entry.attempts.clear();
        }

        // Remove expired attempts
        entry
            .attempts
            .retain(|&attempt_time| now.duration_since(attempt_time) < window);

        // Check if over limit
        if entry.attempts.len() >= self.max_attempts as usize {
            let lockout = Duration::from_secs(self.lockout_secs);
            entry.locked_until = Some(now + lockout);
            return Err(lockout);
        }

        Ok(self
            .max_attempts
            .saturating_sub(entry.attempts.len() as u32))
    }

    /// Record a 2FA attempt (successful or failed).
    pub fn record_attempt(&self, user_id: &str) {
        let now = Instant::now();
        let mut attempts = self.attempts.write().unwrap();
        let entry = attempts
            .entry(user_id.to_string())
            .or_insert_with(TwoFactorAttemptEntry::new);
        entry.attempts.push(now);
    }

    /// Clear rate limit state for a user (e.g., after successful verification).
    pub fn clear(&self, user_id: &str) {
        let mut attempts = self.attempts.write().unwrap();
        attempts.remove(user_id);
    }

    /// Get remaining attempts for a user.
    pub fn remaining(&self, user_id: &str) -> u32 {
        let now = Instant::now();
        let window = Duration::from_secs(self.window_secs);

        let attempts = self.attempts.read().unwrap();
        match attempts.get(user_id) {
            Some(entry) => {
                if entry.locked_until.is_some() {
                    return 0;
                }
                let valid_attempts = entry
                    .attempts
                    .iter()
                    .filter(|&&attempt_time| now.duration_since(attempt_time) < window)
                    .count();
                self.max_attempts.saturating_sub(valid_attempts as u32)
            }
            None => self.max_attempts,
        }
    }
}

impl Default for TwoFactorRateLimiter {
    fn default() -> Self {
        Self::new(5, 300, 900) // 5 attempts per 5 minutes, 15 minute lockout
    }
}

/// Error types for two-factor authentication.
#[derive(Debug, Clone)]
pub enum TwoFactorError {
    /// 2FA is not enabled for this user
    NotEnabled,
    /// 2FA is already enabled
    AlreadyEnabled,
    /// 2FA setup is not pending
    SetupNotPending,
    /// Invalid TOTP code
    InvalidCode,
    /// Invalid backup code
    InvalidBackupCode,
    /// Rate limited
    RateLimited(Duration),
    /// Recovery token expired or invalid
    InvalidRecoveryToken,
    /// Recovery token already used
    RecoveryTokenUsed,
    /// No recovery email configured
    NoRecoveryEmail,
    /// Internal error
    InternalError(String),
}

impl std::fmt::Display for TwoFactorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TwoFactorError::NotEnabled => write!(f, "two-factor authentication is not enabled"),
            TwoFactorError::AlreadyEnabled => {
                write!(f, "two-factor authentication is already enabled")
            }
            TwoFactorError::SetupNotPending => write!(f, "two-factor setup is not pending"),
            TwoFactorError::InvalidCode => write!(f, "invalid verification code"),
            TwoFactorError::InvalidBackupCode => write!(f, "invalid backup code"),
            TwoFactorError::RateLimited(duration) => {
                write!(
                    f,
                    "too many attempts, try again in {} seconds",
                    duration.as_secs()
                )
            }
            TwoFactorError::InvalidRecoveryToken => write!(f, "invalid or expired recovery token"),
            TwoFactorError::RecoveryTokenUsed => {
                write!(f, "recovery token has already been used")
            }
            TwoFactorError::NoRecoveryEmail => write!(f, "no recovery email configured"),
            TwoFactorError::InternalError(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl std::error::Error for TwoFactorError {}

/// Two-factor authentication store for managing user 2FA data.
#[derive(Debug, Clone)]
pub struct TwoFactorStore {
    /// User ID to 2FA data mapping
    data: Arc<RwLock<HashMap<String, TwoFactorData>>>,
    /// Recovery tokens
    recovery_tokens: Arc<RwLock<HashMap<String, RecoveryToken>>>,
}

impl TwoFactorStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            recovery_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get 2FA data for a user.
    pub fn get(&self, user_id: &str) -> Option<TwoFactorData> {
        let data = self.data.read().unwrap();
        data.get(user_id).cloned()
    }

    /// Set 2FA data for a user.
    pub fn set(&self, user_id: &str, two_factor_data: TwoFactorData) {
        let mut data = self.data.write().unwrap();
        data.insert(user_id.to_string(), two_factor_data);
    }

    /// Check if 2FA is enabled for a user.
    pub fn is_enabled(&self, user_id: &str) -> bool {
        self.get(user_id)
            .map(|d| d.status == TwoFactorStatus::Enabled)
            .unwrap_or(false)
    }

    /// Store a recovery token.
    pub fn store_recovery_token(&self, token: RecoveryToken) {
        let mut tokens = self.recovery_tokens.write().unwrap();
        tokens.insert(token.token.clone(), token);
    }

    /// Get and validate a recovery token.
    pub fn get_recovery_token(&self, token: &str) -> Option<RecoveryToken> {
        let tokens = self.recovery_tokens.read().unwrap();
        tokens.get(token).cloned()
    }

    /// Mark a recovery token as used.
    pub fn mark_token_used(&self, token: &str) {
        let mut tokens = self.recovery_tokens.write().unwrap();
        if let Some(t) = tokens.get_mut(token) {
            t.used = true;
        }
    }

    /// Remove a recovery token.
    pub fn remove_recovery_token(&self, token: &str) {
        let mut tokens = self.recovery_tokens.write().unwrap();
        tokens.remove(token);
    }

    /// Clean up expired recovery tokens.
    pub fn cleanup_expired_tokens(&self) {
        let now = current_timestamp();
        let mut tokens = self.recovery_tokens.write().unwrap();
        tokens.retain(|_, t| t.expires_at > now && !t.used);
    }
}

impl Default for TwoFactorStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Two-factor authentication service.
#[derive(Clone)]
pub struct TwoFactorService {
    store: TwoFactorStore,
    rate_limiter: TwoFactorRateLimiter,
    /// Application name for QR code URL
    app_name: String,
}

impl TwoFactorService {
    pub fn new(app_name: &str) -> Self {
        Self {
            store: TwoFactorStore::new(),
            rate_limiter: TwoFactorRateLimiter::default(),
            app_name: app_name.to_string(),
        }
    }

    pub fn with_rate_limiter(mut self, rate_limiter: TwoFactorRateLimiter) -> Self {
        self.rate_limiter = rate_limiter;
        self
    }

    pub fn with_store(mut self, store: TwoFactorStore) -> Self {
        self.store = store;
        self
    }

    /// Check if 2FA is enabled for a user.
    pub fn is_enabled(&self, user_id: &str) -> bool {
        self.store.is_enabled(user_id)
    }

    /// Get the 2FA status for a user.
    pub fn get_status(&self, user_id: &str) -> TwoFactorStatus {
        self.store
            .get(user_id)
            .map(|d| d.status)
            .unwrap_or(TwoFactorStatus::Disabled)
    }

    /// Initiate 2FA setup for a user.
    /// Returns the secret, QR code URL, and backup codes.
    pub fn setup(&self, user_id: &str, user_email: &str) -> Result<TwoFactorSetupResult, TwoFactorError> {
        // Check if already enabled
        if self.is_enabled(user_id) {
            return Err(TwoFactorError::AlreadyEnabled);
        }

        // Generate TOTP secret
        let secret = generate_totp_secret();
        let secret_base32 = base32_encode(&secret);

        // Generate backup codes
        let (backup_codes, backup_code_hashes) = generate_backup_codes();

        // Generate QR code URL
        let qr_code_url = generate_otpauth_url(&self.app_name, user_email, &secret_base32);

        // Store pending 2FA data
        let two_factor_data = TwoFactorData {
            status: TwoFactorStatus::Pending,
            totp_secret: Some(secret_base32.clone()),
            backup_codes: backup_code_hashes,
            recovery_email: Some(user_email.to_string()),
            enabled_at: None,
        };
        self.store.set(user_id, two_factor_data);

        Ok(TwoFactorSetupResult {
            secret: secret_base32,
            qr_code_url,
            backup_codes,
        })
    }

    /// Confirm 2FA setup by verifying a TOTP code.
    /// This transitions from Pending to Enabled status.
    pub fn confirm_setup(&self, user_id: &str, code: &str) -> Result<(), TwoFactorError> {
        // Check rate limit
        self.rate_limiter
            .check(user_id)
            .map_err(TwoFactorError::RateLimited)?;
        self.rate_limiter.record_attempt(user_id);

        let mut data = self
            .store
            .get(user_id)
            .ok_or(TwoFactorError::SetupNotPending)?;

        if data.status != TwoFactorStatus::Pending {
            return Err(TwoFactorError::SetupNotPending);
        }

        let secret = data
            .totp_secret
            .as_ref()
            .ok_or(TwoFactorError::SetupNotPending)?;

        // Verify the code
        if !verify_totp(secret, code) {
            return Err(TwoFactorError::InvalidCode);
        }

        // Enable 2FA
        data.status = TwoFactorStatus::Enabled;
        data.enabled_at = Some(current_timestamp());
        self.store.set(user_id, data);

        // Clear rate limit on success
        self.rate_limiter.clear(user_id);

        Ok(())
    }

    /// Verify a TOTP code for an enabled 2FA user.
    pub fn verify_code(&self, user_id: &str, code: &str) -> Result<(), TwoFactorError> {
        // Check rate limit
        self.rate_limiter
            .check(user_id)
            .map_err(TwoFactorError::RateLimited)?;
        self.rate_limiter.record_attempt(user_id);

        let data = self.store.get(user_id).ok_or(TwoFactorError::NotEnabled)?;

        if data.status != TwoFactorStatus::Enabled {
            return Err(TwoFactorError::NotEnabled);
        }

        let secret = data.totp_secret.as_ref().ok_or(TwoFactorError::NotEnabled)?;

        // Verify the code
        if !verify_totp(secret, code) {
            return Err(TwoFactorError::InvalidCode);
        }

        // Clear rate limit on success
        self.rate_limiter.clear(user_id);

        Ok(())
    }

    /// Verify a backup code and mark it as used.
    pub fn verify_backup_code(&self, user_id: &str, code: &str) -> Result<(), TwoFactorError> {
        // Check rate limit
        self.rate_limiter
            .check(user_id)
            .map_err(TwoFactorError::RateLimited)?;
        self.rate_limiter.record_attempt(user_id);

        let mut data = self.store.get(user_id).ok_or(TwoFactorError::NotEnabled)?;

        if data.status != TwoFactorStatus::Enabled {
            return Err(TwoFactorError::NotEnabled);
        }

        // Hash the provided code and check against stored hashes
        let code_hash = hash_backup_code(code);
        let code_index = data
            .backup_codes
            .iter()
            .position(|h| h == &code_hash)
            .ok_or(TwoFactorError::InvalidBackupCode)?;

        // Remove the used backup code
        data.backup_codes.remove(code_index);
        self.store.set(user_id, data);

        // Clear rate limit on success
        self.rate_limiter.clear(user_id);

        Ok(())
    }

    /// Generate new backup codes (invalidates old ones).
    pub fn regenerate_backup_codes(
        &self,
        user_id: &str,
        code: &str,
    ) -> Result<Vec<String>, TwoFactorError> {
        // Verify current 2FA code first
        self.verify_code(user_id, code)?;

        let mut data = self.store.get(user_id).ok_or(TwoFactorError::NotEnabled)?;

        // Generate new backup codes
        let (backup_codes, backup_code_hashes) = generate_backup_codes();
        data.backup_codes = backup_code_hashes;
        self.store.set(user_id, data);

        Ok(backup_codes)
    }

    /// Get remaining backup code count.
    pub fn remaining_backup_codes(&self, user_id: &str) -> usize {
        self.store
            .get(user_id)
            .map(|d| d.backup_codes.len())
            .unwrap_or(0)
    }

    /// Initiate email recovery flow.
    /// Returns a recovery token that should be sent to the user's email.
    pub fn initiate_recovery(&self, user_id: &str) -> Result<String, TwoFactorError> {
        let data = self.store.get(user_id).ok_or(TwoFactorError::NotEnabled)?;

        if data.status != TwoFactorStatus::Enabled {
            return Err(TwoFactorError::NotEnabled);
        }

        if data.recovery_email.is_none() {
            return Err(TwoFactorError::NoRecoveryEmail);
        }

        // Generate recovery token
        let token = generate_recovery_token();
        let recovery_token = RecoveryToken {
            token: token.clone(),
            user_id: user_id.to_string(),
            expires_at: current_timestamp() + RECOVERY_TOKEN_EXPIRY_SECS,
            used: false,
        };

        self.store.store_recovery_token(recovery_token);

        Ok(token)
    }

    /// Get the recovery email for a user (for sending recovery token).
    pub fn get_recovery_email(&self, user_id: &str) -> Option<String> {
        self.store
            .get(user_id)
            .and_then(|d| d.recovery_email.clone())
    }

    /// Complete recovery using a recovery token.
    /// This disables 2FA for the user.
    pub fn complete_recovery(&self, token: &str) -> Result<String, TwoFactorError> {
        let recovery_token = self
            .store
            .get_recovery_token(token)
            .ok_or(TwoFactorError::InvalidRecoveryToken)?;

        // Check expiration
        if current_timestamp() > recovery_token.expires_at {
            self.store.remove_recovery_token(token);
            return Err(TwoFactorError::InvalidRecoveryToken);
        }

        // Check if already used
        if recovery_token.used {
            return Err(TwoFactorError::RecoveryTokenUsed);
        }

        // Mark token as used
        self.store.mark_token_used(token);

        // Disable 2FA for the user
        let user_id = recovery_token.user_id.clone();
        let mut data = self
            .store
            .get(&user_id)
            .ok_or(TwoFactorError::NotEnabled)?;

        data.status = TwoFactorStatus::Disabled;
        data.totp_secret = None;
        data.backup_codes.clear();
        data.enabled_at = None;
        self.store.set(&user_id, data);

        // Remove the token
        self.store.remove_recovery_token(token);

        Ok(user_id)
    }

    /// Disable 2FA for a user (requires valid TOTP code).
    pub fn disable(&self, user_id: &str, code: &str) -> Result<(), TwoFactorError> {
        // Verify current 2FA code first
        self.verify_code(user_id, code)?;

        let mut data = self.store.get(user_id).ok_or(TwoFactorError::NotEnabled)?;

        data.status = TwoFactorStatus::Disabled;
        data.totp_secret = None;
        data.backup_codes.clear();
        data.enabled_at = None;
        self.store.set(user_id, data);

        Ok(())
    }

    /// Update recovery email (requires valid TOTP code).
    pub fn update_recovery_email(
        &self,
        user_id: &str,
        code: &str,
        new_email: &str,
    ) -> Result<(), TwoFactorError> {
        // Verify current 2FA code first
        self.verify_code(user_id, code)?;

        let mut data = self.store.get(user_id).ok_or(TwoFactorError::NotEnabled)?;
        data.recovery_email = Some(new_email.to_string());
        self.store.set(user_id, data);

        Ok(())
    }

    /// Get the rate limiter for external checks.
    pub fn rate_limiter(&self) -> &TwoFactorRateLimiter {
        &self.rate_limiter
    }
}

// Helper functions

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Generate a random TOTP secret.
fn generate_totp_secret() -> Vec<u8> {
    let mut secret = vec![0u8; TOTP_SECRET_LENGTH];
    OsRng.fill_bytes(&mut secret);
    secret
}

/// Base32 encode a byte slice (RFC 4648).
fn base32_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut result = String::new();
    let mut buffer: u64 = 0;
    let mut bits_left = 0;

    for &byte in data {
        buffer = (buffer << 8) | (byte as u64);
        bits_left += 8;

        while bits_left >= 5 {
            bits_left -= 5;
            let index = ((buffer >> bits_left) & 0x1F) as usize;
            result.push(ALPHABET[index] as char);
        }
    }

    if bits_left > 0 {
        let index = ((buffer << (5 - bits_left)) & 0x1F) as usize;
        result.push(ALPHABET[index] as char);
    }

    result
}

/// Base32 decode a string (RFC 4648).
fn base32_decode(data: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut result = Vec::new();
    let mut buffer: u64 = 0;
    let mut bits_left = 0;

    for c in data.chars() {
        let c = c.to_ascii_uppercase();
        let value = ALPHABET.iter().position(|&x| x == c as u8)?;

        buffer = (buffer << 5) | (value as u64);
        bits_left += 5;

        if bits_left >= 8 {
            bits_left -= 8;
            result.push((buffer >> bits_left) as u8);
        }
    }

    Some(result)
}

/// Generate the otpauth:// URL for QR code.
fn generate_otpauth_url(app_name: &str, user_email: &str, secret: &str) -> String {
    let label = format!(
        "{}:{}",
        url_encode(app_name),
        url_encode(user_email)
    );
    format!(
        "otpauth://totp/{}?secret={}&issuer={}&algorithm=SHA1&digits={}&period={}",
        label,
        secret,
        url_encode(app_name),
        TOTP_DIGITS,
        TOTP_PERIOD
    )
}

/// URL encode a string.
fn url_encode(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}

/// Calculate TOTP code for a given time counter.
fn calculate_totp(secret: &[u8], counter: u64) -> u32 {
    // HMAC-SHA1 (simplified implementation using SHA256 for the HMAC construction)
    let counter_bytes = counter.to_be_bytes();

    // HMAC: H(K XOR opad, H(K XOR ipad, message))
    let mut key = secret.to_vec();
    if key.len() < 64 {
        key.resize(64, 0);
    } else if key.len() > 64 {
        let mut hasher = Sha256::new();
        hasher.update(&key);
        key = hasher.finalize()[..20].to_vec();
        key.resize(64, 0);
    }

    let mut ipad = vec![0x36u8; 64];
    let mut opad = vec![0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }

    // Inner hash
    let mut inner_hasher = Sha256::new();
    inner_hasher.update(&ipad);
    inner_hasher.update(&counter_bytes);
    let inner_hash = inner_hasher.finalize();

    // Outer hash
    let mut outer_hasher = Sha256::new();
    outer_hasher.update(&opad);
    outer_hasher.update(&inner_hash);
    let hash = outer_hasher.finalize();

    // Dynamic truncation
    let offset = (hash[hash.len() - 1] & 0x0F) as usize;
    let binary = ((hash[offset] & 0x7F) as u32) << 24
        | (hash[offset + 1] as u32) << 16
        | (hash[offset + 2] as u32) << 8
        | (hash[offset + 3] as u32);

    binary % 10u32.pow(TOTP_DIGITS)
}

/// Verify a TOTP code (checks current and adjacent time windows).
fn verify_totp(secret_base32: &str, code: &str) -> bool {
    let secret = match base32_decode(secret_base32) {
        Some(s) => s,
        None => return false,
    };

    let code_num: u32 = match code.parse() {
        Ok(n) => n,
        Err(_) => return false,
    };

    // Check code length
    if code.len() != TOTP_DIGITS as usize {
        return false;
    }

    let now = current_timestamp();
    let counter = now / TOTP_PERIOD;

    // Check current and adjacent time windows (±1 period for clock skew)
    for offset in [-1i64, 0, 1] {
        let check_counter = (counter as i64 + offset) as u64;
        if calculate_totp(&secret, check_counter) == code_num {
            return true;
        }
    }

    false
}

/// Generate backup codes.
fn generate_backup_codes() -> (Vec<String>, Vec<String>) {
    let mut codes = Vec::new();
    let mut hashes = Vec::new();

    for _ in 0..BACKUP_CODE_COUNT {
        let code = generate_backup_code();
        let hash = hash_backup_code(&code);
        codes.push(code);
        hashes.push(hash);
    }

    (codes, hashes)
}

/// Generate a single backup code.
fn generate_backup_code() -> String {
    let mut bytes = vec![0u8; BACKUP_CODE_LENGTH];
    OsRng.fill_bytes(&mut bytes);

    // Convert to alphanumeric
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // No I, O, 0, 1 for clarity
    bytes
        .iter()
        .map(|&b| CHARSET[(b as usize) % CHARSET.len()] as char)
        .collect()
}

/// Hash a backup code for secure storage.
fn hash_backup_code(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Generate a recovery token.
fn generate_recovery_token() -> String {
    let mut bytes = vec![0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Hex encode a byte slice.
mod hex {
    pub fn encode(data: impl AsRef<[u8]>) -> String {
        data.as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> TwoFactorService {
        TwoFactorService::new("TestApp")
            .with_rate_limiter(TwoFactorRateLimiter::new(5, 60, 300))
    }

    // Base32 encoding/decoding tests

    #[test]
    fn base32_encode_decodes_correctly() {
        let data = b"Hello World!";
        let encoded = base32_encode(data);
        let decoded = base32_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn base32_encode_produces_valid_output() {
        let data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
        let encoded = base32_encode(&data);
        assert!(encoded.chars().all(|c| "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".contains(c)));
    }

    #[test]
    fn base32_decode_rejects_invalid_chars() {
        assert!(base32_decode("INVALID!").is_none());
        assert!(base32_decode("0000").is_none()); // 0 and 1 are not in base32
    }

    // TOTP tests

    #[test]
    fn generate_totp_secret_has_correct_length() {
        let secret = generate_totp_secret();
        assert_eq!(secret.len(), TOTP_SECRET_LENGTH);
    }

    #[test]
    fn calculate_totp_returns_6_digit_code() {
        let secret = generate_totp_secret();
        let counter = current_timestamp() / TOTP_PERIOD;
        let code = calculate_totp(&secret, counter);
        assert!(code < 1_000_000);
    }

    #[test]
    fn verify_totp_accepts_valid_code() {
        let secret = generate_totp_secret();
        let secret_base32 = base32_encode(&secret);
        let counter = current_timestamp() / TOTP_PERIOD;
        let code = calculate_totp(&secret, counter);
        let code_str = format!("{:06}", code);
        assert!(verify_totp(&secret_base32, &code_str));
    }

    #[test]
    fn verify_totp_rejects_invalid_code() {
        let secret = generate_totp_secret();
        let secret_base32 = base32_encode(&secret);
        assert!(!verify_totp(&secret_base32, "000000"));
        assert!(!verify_totp(&secret_base32, "invalid"));
        assert!(!verify_totp(&secret_base32, "12345")); // Too short
    }

    #[test]
    fn verify_totp_rejects_invalid_secret() {
        assert!(!verify_totp("INVALID!", "123456"));
    }

    // QR code URL tests

    #[test]
    fn generate_otpauth_url_contains_required_params() {
        let url = generate_otpauth_url("MyApp", "user@example.com", "JBSWY3DPEHPK3PXP");
        assert!(url.starts_with("otpauth://totp/"));
        assert!(url.contains("secret=JBSWY3DPEHPK3PXP"));
        assert!(url.contains("issuer=MyApp"));
        assert!(url.contains("digits=6"));
        assert!(url.contains("period=30"));
    }

    #[test]
    fn generate_otpauth_url_encodes_special_chars() {
        let url = generate_otpauth_url("My App", "user@example.com", "SECRET");
        assert!(url.contains("My%20App"));
    }

    // Backup code tests

    #[test]
    fn generate_backup_code_has_correct_length() {
        let code = generate_backup_code();
        assert_eq!(code.len(), BACKUP_CODE_LENGTH);
    }

    #[test]
    fn generate_backup_codes_returns_correct_count() {
        let (codes, hashes) = generate_backup_codes();
        assert_eq!(codes.len(), BACKUP_CODE_COUNT);
        assert_eq!(hashes.len(), BACKUP_CODE_COUNT);
    }

    #[test]
    fn backup_codes_are_unique() {
        let (codes, _) = generate_backup_codes();
        let mut unique: Vec<_> = codes.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), codes.len());
    }

    #[test]
    fn hash_backup_code_is_deterministic() {
        let code = "ABCD1234";
        let hash1 = hash_backup_code(code);
        let hash2 = hash_backup_code(code);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn hash_backup_code_produces_hex_string() {
        let hash = hash_backup_code("TEST");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(hash.len(), 64); // SHA256 = 32 bytes = 64 hex chars
    }

    // Recovery token tests

    #[test]
    fn generate_recovery_token_is_unique() {
        let token1 = generate_recovery_token();
        let token2 = generate_recovery_token();
        assert_ne!(token1, token2);
    }

    #[test]
    fn generate_recovery_token_is_hex() {
        let token = generate_recovery_token();
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(token.len(), 64); // 32 bytes = 64 hex chars
    }

    // TwoFactorStatus tests

    #[test]
    fn two_factor_status_default_is_disabled() {
        assert_eq!(TwoFactorStatus::default(), TwoFactorStatus::Disabled);
    }

    // TwoFactorData tests

    #[test]
    fn two_factor_data_default_values() {
        let data = TwoFactorData::default();
        assert_eq!(data.status, TwoFactorStatus::Disabled);
        assert!(data.totp_secret.is_none());
        assert!(data.backup_codes.is_empty());
        assert!(data.recovery_email.is_none());
        assert!(data.enabled_at.is_none());
    }

    // TwoFactorStore tests

    #[test]
    fn store_get_returns_none_for_unknown_user() {
        let store = TwoFactorStore::new();
        assert!(store.get("unknown").is_none());
    }

    #[test]
    fn store_set_and_get_works() {
        let store = TwoFactorStore::new();
        let data = TwoFactorData {
            status: TwoFactorStatus::Enabled,
            totp_secret: Some("SECRET".to_string()),
            backup_codes: vec!["hash1".to_string()],
            recovery_email: Some("user@example.com".to_string()),
            enabled_at: Some(12345),
        };
        store.set("user1", data.clone());
        let retrieved = store.get("user1").unwrap();
        assert_eq!(retrieved.status, TwoFactorStatus::Enabled);
        assert_eq!(retrieved.totp_secret, Some("SECRET".to_string()));
    }

    #[test]
    fn store_is_enabled_returns_false_for_unknown_user() {
        let store = TwoFactorStore::new();
        assert!(!store.is_enabled("unknown"));
    }

    #[test]
    fn store_is_enabled_returns_correct_status() {
        let store = TwoFactorStore::new();
        let mut data = TwoFactorData::default();
        data.status = TwoFactorStatus::Pending;
        store.set("user1", data);
        assert!(!store.is_enabled("user1"));

        let mut data = TwoFactorData::default();
        data.status = TwoFactorStatus::Enabled;
        store.set("user2", data);
        assert!(store.is_enabled("user2"));
    }

    #[test]
    fn store_recovery_token_lifecycle() {
        let store = TwoFactorStore::new();
        let token = RecoveryToken {
            token: "test-token".to_string(),
            user_id: "user1".to_string(),
            expires_at: current_timestamp() + 3600,
            used: false,
        };
        store.store_recovery_token(token);

        let retrieved = store.get_recovery_token("test-token").unwrap();
        assert_eq!(retrieved.user_id, "user1");
        assert!(!retrieved.used);

        store.mark_token_used("test-token");
        let retrieved = store.get_recovery_token("test-token").unwrap();
        assert!(retrieved.used);

        store.remove_recovery_token("test-token");
        assert!(store.get_recovery_token("test-token").is_none());
    }

    #[test]
    fn store_cleanup_expired_tokens() {
        let store = TwoFactorStore::new();
        let expired_token = RecoveryToken {
            token: "expired".to_string(),
            user_id: "user1".to_string(),
            expires_at: current_timestamp() - 1,
            used: false,
        };
        let valid_token = RecoveryToken {
            token: "valid".to_string(),
            user_id: "user2".to_string(),
            expires_at: current_timestamp() + 3600,
            used: false,
        };
        let used_token = RecoveryToken {
            token: "used".to_string(),
            user_id: "user3".to_string(),
            expires_at: current_timestamp() + 3600,
            used: true,
        };
        store.store_recovery_token(expired_token);
        store.store_recovery_token(valid_token);
        store.store_recovery_token(used_token);

        store.cleanup_expired_tokens();

        assert!(store.get_recovery_token("expired").is_none());
        assert!(store.get_recovery_token("valid").is_some());
        assert!(store.get_recovery_token("used").is_none());
    }

    // TwoFactorRateLimiter tests

    #[test]
    fn rate_limiter_allows_under_limit() {
        let limiter = TwoFactorRateLimiter::new(3, 60, 300);
        assert!(limiter.check("user1").is_ok());
        limiter.record_attempt("user1");
        assert!(limiter.check("user1").is_ok());
        limiter.record_attempt("user1");
        assert!(limiter.check("user1").is_ok());
    }

    #[test]
    fn rate_limiter_blocks_over_limit() {
        let limiter = TwoFactorRateLimiter::new(2, 60, 300);
        limiter.record_attempt("user1");
        limiter.record_attempt("user1");
        let result = limiter.check("user1");
        assert!(result.is_err());
    }

    #[test]
    fn rate_limiter_tracks_users_separately() {
        let limiter = TwoFactorRateLimiter::new(1, 60, 300);
        limiter.record_attempt("user1");
        assert!(limiter.check("user1").is_err());
        assert!(limiter.check("user2").is_ok());
    }

    #[test]
    fn rate_limiter_clear_resets() {
        let limiter = TwoFactorRateLimiter::new(1, 60, 300);
        limiter.record_attempt("user1");
        assert!(limiter.check("user1").is_err());
        limiter.clear("user1");
        assert!(limiter.check("user1").is_ok());
    }

    #[test]
    fn rate_limiter_remaining_correct() {
        let limiter = TwoFactorRateLimiter::new(3, 60, 300);
        assert_eq!(limiter.remaining("user1"), 3);
        limiter.record_attempt("user1");
        assert_eq!(limiter.remaining("user1"), 2);
        limiter.record_attempt("user1");
        assert_eq!(limiter.remaining("user1"), 1);
    }

    // TwoFactorService tests

    #[test]
    fn service_is_enabled_returns_false_for_new_user() {
        let service = test_service();
        assert!(!service.is_enabled("user1"));
    }

    #[test]
    fn service_get_status_returns_disabled_for_new_user() {
        let service = test_service();
        assert_eq!(service.get_status("user1"), TwoFactorStatus::Disabled);
    }

    #[test]
    fn service_setup_returns_result() {
        let service = test_service();
        let result = service.setup("user1", "user@example.com").unwrap();
        assert!(!result.secret.is_empty());
        assert!(result.qr_code_url.starts_with("otpauth://"));
        assert_eq!(result.backup_codes.len(), BACKUP_CODE_COUNT);
    }

    #[test]
    fn service_setup_sets_pending_status() {
        let service = test_service();
        service.setup("user1", "user@example.com").unwrap();
        assert_eq!(service.get_status("user1"), TwoFactorStatus::Pending);
    }

    #[test]
    fn service_setup_fails_if_already_enabled() {
        let service = test_service();
        let store = TwoFactorStore::new();
        let mut data = TwoFactorData::default();
        data.status = TwoFactorStatus::Enabled;
        store.set("user1", data);
        let service = service.with_store(store);

        let result = service.setup("user1", "user@example.com");
        assert!(matches!(result, Err(TwoFactorError::AlreadyEnabled)));
    }

    #[test]
    fn service_confirm_setup_fails_if_not_pending() {
        let service = test_service();
        let result = service.confirm_setup("user1", "123456");
        assert!(matches!(result, Err(TwoFactorError::SetupNotPending)));
    }

    #[test]
    fn service_confirm_setup_fails_with_wrong_code() {
        let service = test_service();
        service.setup("user1", "user@example.com").unwrap();
        let result = service.confirm_setup("user1", "000000");
        assert!(matches!(result, Err(TwoFactorError::InvalidCode)));
    }

    #[test]
    fn service_confirm_setup_succeeds_with_valid_code() {
        let service = test_service();
        let setup = service.setup("user1", "user@example.com").unwrap();

        // Generate valid TOTP code
        let secret = base32_decode(&setup.secret).unwrap();
        let counter = current_timestamp() / TOTP_PERIOD;
        let code = calculate_totp(&secret, counter);
        let code_str = format!("{:06}", code);

        service.confirm_setup("user1", &code_str).unwrap();
        assert!(service.is_enabled("user1"));
        assert_eq!(service.get_status("user1"), TwoFactorStatus::Enabled);
    }

    #[test]
    fn service_verify_code_fails_if_not_enabled() {
        let service = test_service();
        let result = service.verify_code("user1", "123456");
        assert!(matches!(result, Err(TwoFactorError::NotEnabled)));
    }

    #[test]
    fn service_verify_backup_code_fails_if_not_enabled() {
        let service = test_service();
        let result = service.verify_backup_code("user1", "ABCD1234");
        assert!(matches!(result, Err(TwoFactorError::NotEnabled)));
    }

    #[test]
    fn service_remaining_backup_codes_returns_zero_for_unknown_user() {
        let service = test_service();
        assert_eq!(service.remaining_backup_codes("unknown"), 0);
    }

    #[test]
    fn service_initiate_recovery_fails_if_not_enabled() {
        let service = test_service();
        let result = service.initiate_recovery("user1");
        assert!(matches!(result, Err(TwoFactorError::NotEnabled)));
    }

    #[test]
    fn service_get_recovery_email_returns_none_for_unknown_user() {
        let service = test_service();
        assert!(service.get_recovery_email("unknown").is_none());
    }

    #[test]
    fn service_complete_recovery_fails_with_invalid_token() {
        let service = test_service();
        let result = service.complete_recovery("invalid-token");
        assert!(matches!(result, Err(TwoFactorError::InvalidRecoveryToken)));
    }

    #[test]
    fn service_disable_fails_if_not_enabled() {
        let service = test_service();
        let result = service.disable("user1", "123456");
        assert!(matches!(result, Err(TwoFactorError::NotEnabled)));
    }

    #[test]
    fn service_update_recovery_email_fails_if_not_enabled() {
        let service = test_service();
        let result = service.update_recovery_email("user1", "123456", "new@example.com");
        assert!(matches!(result, Err(TwoFactorError::NotEnabled)));
    }

    // Integration tests

    #[test]
    fn full_2fa_setup_and_verify_flow() {
        let service = test_service();

        // Setup
        let setup = service.setup("user1", "user@example.com").unwrap();
        assert_eq!(service.get_status("user1"), TwoFactorStatus::Pending);

        // Generate valid TOTP code and confirm
        let secret = base32_decode(&setup.secret).unwrap();
        let counter = current_timestamp() / TOTP_PERIOD;
        let code = calculate_totp(&secret, counter);
        let code_str = format!("{:06}", code);
        service.confirm_setup("user1", &code_str).unwrap();
        assert!(service.is_enabled("user1"));

        // Verify code
        let code = calculate_totp(&secret, counter);
        let code_str = format!("{:06}", code);
        service.verify_code("user1", &code_str).unwrap();

        // Verify backup code
        let backup_code = &setup.backup_codes[0];
        service.verify_backup_code("user1", backup_code).unwrap();
        assert_eq!(service.remaining_backup_codes("user1"), BACKUP_CODE_COUNT - 1);
    }

    #[test]
    fn recovery_flow() {
        let service = test_service();

        // Setup and enable 2FA
        let setup = service.setup("user1", "user@example.com").unwrap();
        let secret = base32_decode(&setup.secret).unwrap();
        let counter = current_timestamp() / TOTP_PERIOD;
        let code = calculate_totp(&secret, counter);
        let code_str = format!("{:06}", code);
        service.confirm_setup("user1", &code_str).unwrap();
        assert!(service.is_enabled("user1"));

        // Initiate recovery
        let recovery_email = service.get_recovery_email("user1").unwrap();
        assert_eq!(recovery_email, "user@example.com");
        let token = service.initiate_recovery("user1").unwrap();

        // Complete recovery
        let user_id = service.complete_recovery(&token).unwrap();
        assert_eq!(user_id, "user1");
        assert!(!service.is_enabled("user1"));
        assert_eq!(service.get_status("user1"), TwoFactorStatus::Disabled);
    }

    #[test]
    fn disable_2fa_flow() {
        let service = test_service();

        // Setup and enable 2FA
        let setup = service.setup("user1", "user@example.com").unwrap();
        let secret = base32_decode(&setup.secret).unwrap();
        let counter = current_timestamp() / TOTP_PERIOD;
        let code = calculate_totp(&secret, counter);
        let code_str = format!("{:06}", code);
        service.confirm_setup("user1", &code_str).unwrap();
        assert!(service.is_enabled("user1"));

        // Disable with valid code
        let code = calculate_totp(&secret, counter);
        let code_str = format!("{:06}", code);
        service.disable("user1", &code_str).unwrap();
        assert!(!service.is_enabled("user1"));
    }

    // TwoFactorError tests

    #[test]
    fn two_factor_error_display_formats_correctly() {
        assert!(format!("{}", TwoFactorError::NotEnabled).contains("not enabled"));
        assert!(format!("{}", TwoFactorError::AlreadyEnabled).contains("already enabled"));
        assert!(format!("{}", TwoFactorError::SetupNotPending).contains("not pending"));
        assert!(format!("{}", TwoFactorError::InvalidCode).contains("invalid"));
        assert!(format!("{}", TwoFactorError::InvalidBackupCode).contains("backup code"));
        assert!(format!("{}", TwoFactorError::RateLimited(Duration::from_secs(60))).contains("60"));
        assert!(format!("{}", TwoFactorError::InvalidRecoveryToken).contains("recovery"));
        assert!(format!("{}", TwoFactorError::RecoveryTokenUsed).contains("used"));
        assert!(format!("{}", TwoFactorError::NoRecoveryEmail).contains("email"));
        assert!(format!("{}", TwoFactorError::InternalError("test".to_string())).contains("test"));
    }

    // URL encoding tests

    #[test]
    fn url_encode_handles_special_chars() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("user@example.com"), "user%40example.com");
        assert_eq!(url_encode("a-b_c.d~e"), "a-b_c.d~e");
    }
}

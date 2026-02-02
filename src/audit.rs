//! Audit logging module for recording authenticated actions.
//!
//! This module provides:
//! - Audit entry recording with timestamp, actor, action, and resource
//! - In-memory audit log store with configurable retention
//! - Query API with filtering by time range, actor, action, and resource
//! - Thread-safe access for concurrent requests

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Action types that can be audited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // Authentication actions
    UserRegister,
    UserLogin,
    UserLogout,
    TokenRefresh,

    // Session actions
    SessionCreate,
    SessionStop,
    SessionView,

    // Task actions
    TaskView,
    TaskUpdate,

    // Log actions
    LogsView,

    // Orchestration actions
    OrchestrationView,

    // User management actions
    UserView,
    UserCreate,
    UserUpdate,
    UserDelete,
    RoleChange,

    // Organization actions
    OrgCreate,
    OrgUpdate,
    OrgDelete,
    OrgMemberAdd,
    OrgMemberRemove,

    // System actions
    ConfigChange,
    AuditLogQuery,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditAction::UserRegister => write!(f, "user.register"),
            AuditAction::UserLogin => write!(f, "user.login"),
            AuditAction::UserLogout => write!(f, "user.logout"),
            AuditAction::TokenRefresh => write!(f, "token.refresh"),
            AuditAction::SessionCreate => write!(f, "session.create"),
            AuditAction::SessionStop => write!(f, "session.stop"),
            AuditAction::SessionView => write!(f, "session.view"),
            AuditAction::TaskView => write!(f, "task.view"),
            AuditAction::TaskUpdate => write!(f, "task.update"),
            AuditAction::LogsView => write!(f, "logs.view"),
            AuditAction::OrchestrationView => write!(f, "orchestration.view"),
            AuditAction::UserView => write!(f, "user.view"),
            AuditAction::UserCreate => write!(f, "user.create"),
            AuditAction::UserUpdate => write!(f, "user.update"),
            AuditAction::UserDelete => write!(f, "user.delete"),
            AuditAction::RoleChange => write!(f, "role.change"),
            AuditAction::OrgCreate => write!(f, "org.create"),
            AuditAction::OrgUpdate => write!(f, "org.update"),
            AuditAction::OrgDelete => write!(f, "org.delete"),
            AuditAction::OrgMemberAdd => write!(f, "org.member.add"),
            AuditAction::OrgMemberRemove => write!(f, "org.member.remove"),
            AuditAction::ConfigChange => write!(f, "config.change"),
            AuditAction::AuditLogQuery => write!(f, "audit.query"),
        }
    }
}

/// Outcome of an audited action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditOutcome {
    Success,
    Failure,
    Denied,
}

impl std::fmt::Display for AuditOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditOutcome::Success => write!(f, "success"),
            AuditOutcome::Failure => write!(f, "failure"),
            AuditOutcome::Denied => write!(f, "denied"),
        }
    }
}

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID
    pub id: String,
    /// Unix timestamp when the action occurred
    pub timestamp: u64,
    /// User ID who performed the action (None for unauthenticated actions)
    pub actor_id: Option<String>,
    /// Email of the actor (for display purposes)
    pub actor_email: Option<String>,
    /// IP address of the client
    pub client_ip: String,
    /// Action performed
    pub action: AuditAction,
    /// Resource type affected (e.g., "session", "task", "user")
    pub resource_type: Option<String>,
    /// Resource ID affected
    pub resource_id: Option<String>,
    /// Outcome of the action
    pub outcome: AuditOutcome,
    /// Additional details or error message
    pub details: Option<String>,
    /// User agent string
    pub user_agent: Option<String>,
}

impl AuditEntry {
    /// Create a new audit entry with the current timestamp.
    pub fn new(
        actor_id: Option<String>,
        actor_email: Option<String>,
        client_ip: String,
        action: AuditAction,
        outcome: AuditOutcome,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp,
            actor_id,
            actor_email,
            client_ip,
            action,
            resource_type: None,
            resource_id: None,
            outcome,
            details: None,
            user_agent: None,
        }
    }

    /// Set the resource affected by this action.
    pub fn with_resource(mut self, resource_type: &str, resource_id: &str) -> Self {
        self.resource_type = Some(resource_type.to_string());
        self.resource_id = Some(resource_id.to_string());
        self
    }

    /// Set additional details for this entry.
    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }

    /// Set the user agent for this entry.
    pub fn with_user_agent(mut self, user_agent: &str) -> Self {
        self.user_agent = Some(user_agent.to_string());
        self
    }
}

/// Configuration for audit log retention.
#[derive(Debug, Clone)]
pub struct AuditLogConfig {
    /// Maximum number of entries to retain
    pub max_entries: usize,
    /// Maximum age of entries in seconds (0 = no age limit)
    pub max_age_secs: u64,
    /// How often to run cleanup (in number of writes)
    pub cleanup_interval: usize,
}

impl Default for AuditLogConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            max_age_secs: 30 * 24 * 60 * 60, // 30 days
            cleanup_interval: 100,
        }
    }
}

impl AuditLogConfig {
    /// Create a new config with custom max entries.
    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries;
        self
    }

    /// Create a new config with custom max age.
    pub fn with_max_age_secs(mut self, max_age_secs: u64) -> Self {
        self.max_age_secs = max_age_secs;
        self
    }

    /// Create a new config with custom cleanup interval.
    pub fn with_cleanup_interval(mut self, cleanup_interval: usize) -> Self {
        self.cleanup_interval = cleanup_interval;
        self
    }
}

/// Query parameters for filtering audit logs.
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    /// Filter by actor ID
    pub actor_id: Option<String>,
    /// Filter by action type
    pub action: Option<AuditAction>,
    /// Filter by resource type
    pub resource_type: Option<String>,
    /// Filter by resource ID
    pub resource_id: Option<String>,
    /// Filter by outcome
    pub outcome: Option<AuditOutcome>,
    /// Filter by minimum timestamp (inclusive)
    pub from_timestamp: Option<u64>,
    /// Filter by maximum timestamp (inclusive)
    pub to_timestamp: Option<u64>,
    /// Maximum number of results to return
    pub limit: Option<usize>,
    /// Number of results to skip (for pagination)
    pub offset: Option<usize>,
}

impl AuditQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_actor_id(mut self, actor_id: &str) -> Self {
        self.actor_id = Some(actor_id.to_string());
        self
    }

    pub fn with_action(mut self, action: AuditAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn with_resource_type(mut self, resource_type: &str) -> Self {
        self.resource_type = Some(resource_type.to_string());
        self
    }

    pub fn with_resource_id(mut self, resource_id: &str) -> Self {
        self.resource_id = Some(resource_id.to_string());
        self
    }

    pub fn with_outcome(mut self, outcome: AuditOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    pub fn with_time_range(mut self, from: u64, to: u64) -> Self {
        self.from_timestamp = Some(from);
        self.to_timestamp = Some(to);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Check if an entry matches this query's filters.
    fn matches(&self, entry: &AuditEntry) -> bool {
        if let Some(ref actor_id) = self.actor_id {
            if entry.actor_id.as_ref() != Some(actor_id) {
                return false;
            }
        }

        if let Some(action) = self.action {
            if entry.action != action {
                return false;
            }
        }

        if let Some(ref resource_type) = self.resource_type {
            if entry.resource_type.as_ref() != Some(resource_type) {
                return false;
            }
        }

        if let Some(ref resource_id) = self.resource_id {
            if entry.resource_id.as_ref() != Some(resource_id) {
                return false;
            }
        }

        if let Some(outcome) = self.outcome {
            if entry.outcome != outcome {
                return false;
            }
        }

        if let Some(from) = self.from_timestamp {
            if entry.timestamp < from {
                return false;
            }
        }

        if let Some(to) = self.to_timestamp {
            if entry.timestamp > to {
                return false;
            }
        }

        true
    }
}

/// Result of an audit log query.
#[derive(Debug, Clone, Serialize)]
pub struct AuditQueryResult {
    /// Matching entries
    pub entries: Vec<AuditEntry>,
    /// Total number of matching entries (before pagination)
    pub total_count: usize,
    /// Whether there are more results
    pub has_more: bool,
}

/// Thread-safe audit log store.
#[derive(Clone)]
pub struct AuditLog {
    config: AuditLogConfig,
    entries: Arc<RwLock<VecDeque<AuditEntry>>>,
    write_count: Arc<RwLock<usize>>,
}

impl AuditLog {
    /// Create a new audit log with the given configuration.
    pub fn new(config: AuditLogConfig) -> Self {
        Self {
            config,
            entries: Arc::new(RwLock::new(VecDeque::new())),
            write_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Record an audit entry.
    pub fn record(&self, entry: AuditEntry) {
        let mut entries = self.entries.write().unwrap();
        entries.push_back(entry);

        // Update write count and maybe run cleanup
        let mut write_count = self.write_count.write().unwrap();
        *write_count += 1;

        if *write_count >= self.config.cleanup_interval {
            *write_count = 0;
            drop(write_count);
            self.cleanup_locked(&mut entries);
        }
    }

    /// Query the audit log with filters.
    pub fn query(&self, query: &AuditQuery) -> AuditQueryResult {
        let entries = self.entries.read().unwrap();

        // Filter entries
        let matching: Vec<AuditEntry> = entries
            .iter()
            .filter(|e| query.matches(e))
            .cloned()
            .collect();

        let total_count = matching.len();

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(100).min(1000); // Cap at 1000

        let paginated: Vec<AuditEntry> = matching
            .into_iter()
            .rev() // Most recent first
            .skip(offset)
            .take(limit)
            .collect();

        let has_more = offset + paginated.len() < total_count;

        AuditQueryResult {
            entries: paginated,
            total_count,
            has_more,
        }
    }

    /// Get the total number of entries in the log.
    pub fn count(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    /// Get statistics about the audit log.
    pub fn stats(&self) -> AuditLogStats {
        let entries = self.entries.read().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let oldest_timestamp = entries.front().map(|e| e.timestamp);
        let newest_timestamp = entries.back().map(|e| e.timestamp);

        AuditLogStats {
            total_entries: entries.len(),
            max_entries: self.config.max_entries,
            retention_days: self.config.max_age_secs / (24 * 60 * 60),
            oldest_entry_age_secs: oldest_timestamp.map(|t| now.saturating_sub(t)),
            newest_entry_age_secs: newest_timestamp.map(|t| now.saturating_sub(t)),
        }
    }

    /// Force a cleanup of old entries.
    pub fn cleanup(&self) {
        let mut entries = self.entries.write().unwrap();
        self.cleanup_locked(&mut entries);
    }

    fn cleanup_locked(&self, entries: &mut VecDeque<AuditEntry>) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Remove entries older than max_age_secs
        if self.config.max_age_secs > 0 {
            let cutoff = now.saturating_sub(self.config.max_age_secs);
            while let Some(entry) = entries.front() {
                if entry.timestamp < cutoff {
                    entries.pop_front();
                } else {
                    break;
                }
            }
        }

        // Remove entries if over max_entries limit
        while entries.len() > self.config.max_entries {
            entries.pop_front();
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &AuditLogConfig {
        &self.config
    }

    /// Update the retention configuration.
    pub fn update_config(&mut self, config: AuditLogConfig) {
        self.config = config;
        self.cleanup();
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new(AuditLogConfig::default())
    }
}

/// Statistics about the audit log.
#[derive(Debug, Clone, Serialize)]
pub struct AuditLogStats {
    pub total_entries: usize,
    pub max_entries: usize,
    pub retention_days: u64,
    pub oldest_entry_age_secs: Option<u64>,
    pub newest_entry_age_secs: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn audit_entry_new_sets_timestamp() {
        let entry = AuditEntry::new(
            Some("user-1".to_string()),
            Some("test@example.com".to_string()),
            "127.0.0.1".to_string(),
            AuditAction::UserLogin,
            AuditOutcome::Success,
        );

        assert!(!entry.id.is_empty());
        assert!(entry.timestamp > 0);
        assert_eq!(entry.actor_id, Some("user-1".to_string()));
        assert_eq!(entry.actor_email, Some("test@example.com".to_string()));
        assert_eq!(entry.client_ip, "127.0.0.1");
        assert_eq!(entry.action, AuditAction::UserLogin);
        assert_eq!(entry.outcome, AuditOutcome::Success);
    }

    #[test]
    fn audit_entry_with_resource() {
        let entry = AuditEntry::new(
            Some("user-1".to_string()),
            None,
            "127.0.0.1".to_string(),
            AuditAction::SessionStop,
            AuditOutcome::Success,
        )
        .with_resource("session", "session-123");

        assert_eq!(entry.resource_type, Some("session".to_string()));
        assert_eq!(entry.resource_id, Some("session-123".to_string()));
    }

    #[test]
    fn audit_entry_with_details() {
        let entry = AuditEntry::new(
            None,
            None,
            "127.0.0.1".to_string(),
            AuditAction::UserLogin,
            AuditOutcome::Failure,
        )
        .with_details("Invalid credentials");

        assert_eq!(entry.details, Some("Invalid credentials".to_string()));
    }

    #[test]
    fn audit_entry_with_user_agent() {
        let entry = AuditEntry::new(
            None,
            None,
            "127.0.0.1".to_string(),
            AuditAction::UserLogin,
            AuditOutcome::Success,
        )
        .with_user_agent("Mozilla/5.0");

        assert_eq!(entry.user_agent, Some("Mozilla/5.0".to_string()));
    }

    #[test]
    fn audit_log_record_and_count() {
        let log = AuditLog::new(AuditLogConfig::default());

        log.record(AuditEntry::new(
            Some("user-1".to_string()),
            None,
            "127.0.0.1".to_string(),
            AuditAction::UserLogin,
            AuditOutcome::Success,
        ));

        assert_eq!(log.count(), 1);

        log.record(AuditEntry::new(
            Some("user-2".to_string()),
            None,
            "127.0.0.2".to_string(),
            AuditAction::UserLogout,
            AuditOutcome::Success,
        ));

        assert_eq!(log.count(), 2);
    }

    #[test]
    fn audit_log_query_all() {
        let log = AuditLog::new(AuditLogConfig::default());

        for i in 0..5 {
            log.record(AuditEntry::new(
                Some(format!("user-{}", i)),
                None,
                "127.0.0.1".to_string(),
                AuditAction::UserLogin,
                AuditOutcome::Success,
            ));
        }

        let result = log.query(&AuditQuery::new());
        assert_eq!(result.total_count, 5);
        assert_eq!(result.entries.len(), 5);
        assert!(!result.has_more);
    }

    #[test]
    fn audit_log_query_by_actor() {
        let log = AuditLog::new(AuditLogConfig::default());

        log.record(AuditEntry::new(
            Some("user-1".to_string()),
            None,
            "127.0.0.1".to_string(),
            AuditAction::UserLogin,
            AuditOutcome::Success,
        ));
        log.record(AuditEntry::new(
            Some("user-2".to_string()),
            None,
            "127.0.0.1".to_string(),
            AuditAction::UserLogin,
            AuditOutcome::Success,
        ));
        log.record(AuditEntry::new(
            Some("user-1".to_string()),
            None,
            "127.0.0.1".to_string(),
            AuditAction::UserLogout,
            AuditOutcome::Success,
        ));

        let result = log.query(&AuditQuery::new().with_actor_id("user-1"));
        assert_eq!(result.total_count, 2);
        assert!(result.entries.iter().all(|e| e.actor_id == Some("user-1".to_string())));
    }

    #[test]
    fn audit_log_query_by_action() {
        let log = AuditLog::new(AuditLogConfig::default());

        log.record(AuditEntry::new(
            None,
            None,
            "127.0.0.1".to_string(),
            AuditAction::UserLogin,
            AuditOutcome::Success,
        ));
        log.record(AuditEntry::new(
            None,
            None,
            "127.0.0.1".to_string(),
            AuditAction::SessionCreate,
            AuditOutcome::Success,
        ));
        log.record(AuditEntry::new(
            None,
            None,
            "127.0.0.1".to_string(),
            AuditAction::UserLogin,
            AuditOutcome::Failure,
        ));

        let result = log.query(&AuditQuery::new().with_action(AuditAction::UserLogin));
        assert_eq!(result.total_count, 2);
        assert!(result.entries.iter().all(|e| e.action == AuditAction::UserLogin));
    }

    #[test]
    fn audit_log_query_by_outcome() {
        let log = AuditLog::new(AuditLogConfig::default());

        log.record(AuditEntry::new(
            None,
            None,
            "127.0.0.1".to_string(),
            AuditAction::UserLogin,
            AuditOutcome::Success,
        ));
        log.record(AuditEntry::new(
            None,
            None,
            "127.0.0.1".to_string(),
            AuditAction::UserLogin,
            AuditOutcome::Failure,
        ));
        log.record(AuditEntry::new(
            None,
            None,
            "127.0.0.1".to_string(),
            AuditAction::SessionStop,
            AuditOutcome::Denied,
        ));

        let result = log.query(&AuditQuery::new().with_outcome(AuditOutcome::Success));
        assert_eq!(result.total_count, 1);
        assert!(result.entries.iter().all(|e| e.outcome == AuditOutcome::Success));
    }

    #[test]
    fn audit_log_query_by_resource() {
        let log = AuditLog::new(AuditLogConfig::default());

        log.record(
            AuditEntry::new(
                None,
                None,
                "127.0.0.1".to_string(),
                AuditAction::SessionStop,
                AuditOutcome::Success,
            )
            .with_resource("session", "session-1"),
        );
        log.record(
            AuditEntry::new(
                None,
                None,
                "127.0.0.1".to_string(),
                AuditAction::TaskUpdate,
                AuditOutcome::Success,
            )
            .with_resource("task", "task-1"),
        );
        log.record(
            AuditEntry::new(
                None,
                None,
                "127.0.0.1".to_string(),
                AuditAction::SessionStop,
                AuditOutcome::Success,
            )
            .with_resource("session", "session-2"),
        );

        let result = log.query(&AuditQuery::new().with_resource_type("session"));
        assert_eq!(result.total_count, 2);

        let result = log.query(
            &AuditQuery::new()
                .with_resource_type("session")
                .with_resource_id("session-1"),
        );
        assert_eq!(result.total_count, 1);
    }

    #[test]
    fn audit_log_query_pagination() {
        let log = AuditLog::new(AuditLogConfig::default());

        for i in 0..10 {
            let mut entry = AuditEntry::new(
                Some(format!("user-{}", i)),
                None,
                "127.0.0.1".to_string(),
                AuditAction::UserLogin,
                AuditOutcome::Success,
            );
            // Manually set distinct timestamps for ordering
            entry.timestamp = 1000 + i as u64;
            log.record(entry);
        }

        // First page
        let result = log.query(&AuditQuery::new().with_limit(3).with_offset(0));
        assert_eq!(result.entries.len(), 3);
        assert_eq!(result.total_count, 10);
        assert!(result.has_more);

        // Second page
        let result = log.query(&AuditQuery::new().with_limit(3).with_offset(3));
        assert_eq!(result.entries.len(), 3);
        assert!(result.has_more);

        // Last page
        let result = log.query(&AuditQuery::new().with_limit(3).with_offset(9));
        assert_eq!(result.entries.len(), 1);
        assert!(!result.has_more);
    }

    #[test]
    fn audit_log_query_time_range() {
        let log = AuditLog::new(AuditLogConfig::default());

        for i in 0..5 {
            let mut entry = AuditEntry::new(
                None,
                None,
                "127.0.0.1".to_string(),
                AuditAction::UserLogin,
                AuditOutcome::Success,
            );
            entry.timestamp = 1000 + i * 100;
            log.record(entry);
        }

        let result = log.query(&AuditQuery::new().with_time_range(1100, 1300));
        assert_eq!(result.total_count, 3); // 1100, 1200, 1300
    }

    #[test]
    fn audit_log_cleanup_by_max_entries() {
        let config = AuditLogConfig::default()
            .with_max_entries(5)
            .with_cleanup_interval(1);
        let log = AuditLog::new(config);

        for i in 0..10 {
            log.record(AuditEntry::new(
                Some(format!("user-{}", i)),
                None,
                "127.0.0.1".to_string(),
                AuditAction::UserLogin,
                AuditOutcome::Success,
            ));
        }

        assert!(log.count() <= 5);
    }

    #[test]
    fn audit_log_cleanup_by_age() {
        let config = AuditLogConfig::default()
            .with_max_age_secs(60)
            .with_cleanup_interval(1);
        let log = AuditLog::new(config);

        // Add old entries
        for i in 0..3 {
            let mut entry = AuditEntry::new(
                Some(format!("old-user-{}", i)),
                None,
                "127.0.0.1".to_string(),
                AuditAction::UserLogin,
                AuditOutcome::Success,
            );
            entry.timestamp = 1; // Very old
            log.record(entry);
        }

        // Add recent entries
        for i in 0..3 {
            log.record(AuditEntry::new(
                Some(format!("new-user-{}", i)),
                None,
                "127.0.0.1".to_string(),
                AuditAction::UserLogin,
                AuditOutcome::Success,
            ));
        }

        // After cleanup, old entries should be removed
        assert!(log.count() <= 3);
    }

    #[test]
    fn audit_log_stats() {
        let log = AuditLog::new(
            AuditLogConfig::default()
                .with_max_entries(1000)
                .with_max_age_secs(7 * 24 * 60 * 60),
        );

        log.record(AuditEntry::new(
            None,
            None,
            "127.0.0.1".to_string(),
            AuditAction::UserLogin,
            AuditOutcome::Success,
        ));

        let stats = log.stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.max_entries, 1000);
        assert_eq!(stats.retention_days, 7);
        assert!(stats.oldest_entry_age_secs.is_some());
        assert!(stats.newest_entry_age_secs.is_some());
    }

    #[test]
    fn audit_log_thread_safety() {
        let log = AuditLog::new(AuditLogConfig::default());
        let log1 = log.clone();
        let log2 = log.clone();

        let h1 = thread::spawn(move || {
            for i in 0..100 {
                log1.record(AuditEntry::new(
                    Some(format!("thread1-user-{}", i)),
                    None,
                    "127.0.0.1".to_string(),
                    AuditAction::UserLogin,
                    AuditOutcome::Success,
                ));
            }
        });

        let h2 = thread::spawn(move || {
            for i in 0..100 {
                log2.record(AuditEntry::new(
                    Some(format!("thread2-user-{}", i)),
                    None,
                    "127.0.0.2".to_string(),
                    AuditAction::UserLogout,
                    AuditOutcome::Success,
                ));
            }
        });

        h1.join().unwrap();
        h2.join().unwrap();

        assert_eq!(log.count(), 200);
    }

    #[test]
    fn audit_action_display() {
        assert_eq!(format!("{}", AuditAction::UserLogin), "user.login");
        assert_eq!(format!("{}", AuditAction::SessionStop), "session.stop");
        assert_eq!(format!("{}", AuditAction::TaskUpdate), "task.update");
        assert_eq!(format!("{}", AuditAction::AuditLogQuery), "audit.query");
    }

    #[test]
    fn audit_outcome_display() {
        assert_eq!(format!("{}", AuditOutcome::Success), "success");
        assert_eq!(format!("{}", AuditOutcome::Failure), "failure");
        assert_eq!(format!("{}", AuditOutcome::Denied), "denied");
    }

    #[test]
    fn audit_query_combined_filters() {
        let log = AuditLog::new(AuditLogConfig::default());

        // Add various entries
        log.record(
            AuditEntry::new(
                Some("user-1".to_string()),
                None,
                "127.0.0.1".to_string(),
                AuditAction::SessionStop,
                AuditOutcome::Success,
            )
            .with_resource("session", "session-1"),
        );
        log.record(
            AuditEntry::new(
                Some("user-1".to_string()),
                None,
                "127.0.0.1".to_string(),
                AuditAction::SessionStop,
                AuditOutcome::Failure,
            )
            .with_resource("session", "session-2"),
        );
        log.record(
            AuditEntry::new(
                Some("user-2".to_string()),
                None,
                "127.0.0.1".to_string(),
                AuditAction::SessionStop,
                AuditOutcome::Success,
            )
            .with_resource("session", "session-3"),
        );

        // Query with combined filters
        let result = log.query(
            &AuditQuery::new()
                .with_actor_id("user-1")
                .with_action(AuditAction::SessionStop)
                .with_outcome(AuditOutcome::Success),
        );

        assert_eq!(result.total_count, 1);
        assert_eq!(
            result.entries[0].resource_id,
            Some("session-1".to_string())
        );
    }

    #[test]
    fn audit_log_config_builder() {
        let config = AuditLogConfig::default()
            .with_max_entries(5000)
            .with_max_age_secs(60 * 60 * 24 * 7)
            .with_cleanup_interval(50);

        assert_eq!(config.max_entries, 5000);
        assert_eq!(config.max_age_secs, 60 * 60 * 24 * 7);
        assert_eq!(config.cleanup_interval, 50);
    }

    #[test]
    fn audit_log_update_config() {
        let mut log = AuditLog::new(AuditLogConfig::default().with_max_entries(100));

        for i in 0..50 {
            log.record(AuditEntry::new(
                Some(format!("user-{}", i)),
                None,
                "127.0.0.1".to_string(),
                AuditAction::UserLogin,
                AuditOutcome::Success,
            ));
        }

        assert_eq!(log.count(), 50);

        // Reduce max entries
        log.update_config(AuditLogConfig::default().with_max_entries(20));

        assert!(log.count() <= 20);
    }

    #[test]
    fn audit_query_limit_capped_at_1000() {
        let log = AuditLog::new(AuditLogConfig::default());

        for i in 0..1500 {
            log.record(AuditEntry::new(
                Some(format!("user-{}", i)),
                None,
                "127.0.0.1".to_string(),
                AuditAction::UserLogin,
                AuditOutcome::Success,
            ));
        }

        let result = log.query(&AuditQuery::new().with_limit(2000));
        assert!(result.entries.len() <= 1000);
    }

    #[test]
    fn audit_query_results_ordered_newest_first() {
        let log = AuditLog::new(AuditLogConfig::default());

        for i in 0..5 {
            let mut entry = AuditEntry::new(
                Some(format!("user-{}", i)),
                None,
                "127.0.0.1".to_string(),
                AuditAction::UserLogin,
                AuditOutcome::Success,
            );
            entry.timestamp = 1000 + i as u64;
            log.record(entry);
        }

        let result = log.query(&AuditQuery::new());
        assert_eq!(result.entries.len(), 5);

        // Verify newest first ordering
        for i in 0..4 {
            assert!(result.entries[i].timestamp >= result.entries[i + 1].timestamp);
        }
    }
}

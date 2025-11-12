use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// GitHub notification thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub repository: NotificationRepository,
    pub subject: NotificationSubject,
    pub reason: String,
    pub unread: bool,
    pub updated_at: DateTime<Utc>,
    pub last_read_at: Option<DateTime<Utc>>,
    pub url: String,
}

/// Minimal repository info in notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRepository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub owner: NotificationOwner,
    #[serde(default)]
    pub private: bool,
    pub html_url: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Repository owner in notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationOwner {
    pub login: String,
    pub avatar_url: String,
}

/// Subject of the notification (Issue, PR, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSubject {
    pub title: String,
    #[serde(rename = "type")]
    pub subject_type: String, // "Issue", "PullRequest", "Commit", "Release"
    pub url: Option<String>,
    pub latest_comment_url: Option<String>,
}

/// Notification reason types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationReason {
    Assign,           // Assigned to you
    Author,           // You're the author
    Comment,          // Commented on
    Invitation,       // Invited to contribute
    Manual,           // Manually subscribed
    Mention,          // Mentioned you
    ReviewRequested,  // Review requested
    SecurityAlert,    // Security vulnerability
    StateChange,      // Issue/PR state changed
    Subscribed,       // Watching the repo
    TeamMention,      // Team mentioned
    #[serde(other)]
    Other,
}

impl std::fmt::Display for NotificationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationReason::Assign => write!(f, "Assigned"),
            NotificationReason::Author => write!(f, "Author"),
            NotificationReason::Comment => write!(f, "Comment"),
            NotificationReason::Invitation => write!(f, "Invitation"),
            NotificationReason::Manual => write!(f, "Manual"),
            NotificationReason::Mention => write!(f, "Mention"),
            NotificationReason::ReviewRequested => write!(f, "Review"),
            NotificationReason::SecurityAlert => write!(f, "Security"),
            NotificationReason::StateChange => write!(f, "State Change"),
            NotificationReason::Subscribed => write!(f, "Subscribed"),
            NotificationReason::TeamMention => write!(f, "Team Mention"),
            NotificationReason::Other => write!(f, "Other"),
        }
    }
}

/// Filters for notification queries
#[derive(Debug, Clone, Default)]
pub struct NotificationFilters {
    /// Only show unread notifications
    pub unread_only: bool,
    /// Filter by repository (owner/repo)
    pub repository: Option<String>,
    /// Filter by reason
    pub reason: Option<NotificationReason>,
    /// Filter by subject type (Issue, PullRequest, etc.)
    pub subject_type: Option<String>,
    /// Show only participating (exclude watching notifications)
    pub participating: bool,
}

impl NotificationFilters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn unread_only(mut self) -> Self {
        self.unread_only = true;
        self
    }

    pub fn repository(mut self, repo: String) -> Self {
        self.repository = Some(repo);
        self
    }

    pub fn reason(mut self, reason: NotificationReason) -> Self {
        self.reason = Some(reason);
        self
    }

    pub fn participating(mut self) -> Self {
        self.participating = true;
        self
    }
}

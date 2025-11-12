use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Repository health metrics and scoring
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthMetrics {
    /// Overall health score (0-100)
    pub score: u8,
    /// Health status category
    pub status: HealthStatus,
    /// Maintenance level indicator
    pub maintenance: MaintenanceLevel,
    /// Individual metric scores
    pub metrics: DetailedMetrics,
}

/// Overall health status categories
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    /// Score 80-100: Active, well-maintained
    Healthy,
    /// Score 60-79: Moderately active
    Moderate,
    /// Score 40-59: Low activity
    Warning,
    /// Score 0-39: Potentially abandoned
    Critical,
}

impl HealthStatus {
    pub fn from_score(score: u8) -> Self {
        match score {
            80..=100 => HealthStatus::Healthy,
            60..=79 => HealthStatus::Moderate,
            40..=59 => HealthStatus::Warning,
            _ => HealthStatus::Critical,
        }
    }

    pub fn color_code(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "green",
            HealthStatus::Moderate => "yellow",
            HealthStatus::Warning => "orange",
            HealthStatus::Critical => "red",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "✓",
            HealthStatus::Moderate => "○",
            HealthStatus::Warning => "!",
            HealthStatus::Critical => "✗",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "Healthy",
            HealthStatus::Moderate => "Moderate",
            HealthStatus::Warning => "Warning",
            HealthStatus::Critical => "Critical",
        }
    }
}

/// Maintenance activity level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MaintenanceLevel {
    /// Active development (pushed within 30 days)
    Active,
    /// Maintained (pushed within 90 days)
    Maintained,
    /// Stale (pushed within 180 days)
    Stale,
    /// Inactive (pushed within 365 days)
    Inactive,
    /// Abandoned (pushed > 365 days ago)
    Abandoned,
}

impl MaintenanceLevel {
    pub fn from_last_push(last_push: DateTime<Utc>, now: DateTime<Utc>) -> Self {
        let days_since_push = (now - last_push).num_days();

        match days_since_push {
            0..=30 => MaintenanceLevel::Active,
            31..=90 => MaintenanceLevel::Maintained,
            91..=180 => MaintenanceLevel::Stale,
            181..=365 => MaintenanceLevel::Inactive,
            _ => MaintenanceLevel::Abandoned,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            MaintenanceLevel::Active => "Active",
            MaintenanceLevel::Maintained => "Maintained",
            MaintenanceLevel::Stale => "Stale",
            MaintenanceLevel::Inactive => "Inactive",
            MaintenanceLevel::Abandoned => "Abandoned",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            MaintenanceLevel::Active => "Recently updated (< 30 days)",
            MaintenanceLevel::Maintained => "Regularly maintained (< 90 days)",
            MaintenanceLevel::Stale => "Infrequently updated (< 6 months)",
            MaintenanceLevel::Inactive => "Rarely updated (< 1 year)",
            MaintenanceLevel::Abandoned => "No recent activity (> 1 year)",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            MaintenanceLevel::Active => "🔥",
            MaintenanceLevel::Maintained => "✓",
            MaintenanceLevel::Stale => "⚠",
            MaintenanceLevel::Inactive => "⏸",
            MaintenanceLevel::Abandoned => "💀",
        }
    }
}

/// Detailed breakdown of health metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetailedMetrics {
    /// Activity score (0-30): Based on push frequency
    pub activity_score: u8,
    /// Community score (0-25): Based on stars, forks, watchers
    pub community_score: u8,
    /// Responsiveness score (0-20): Based on open issues ratio
    pub responsiveness_score: u8,
    /// Maturity score (0-15): Based on repository age
    pub maturity_score: u8,
    /// Documentation score (0-10): Has README, description, topics
    pub documentation_score: u8,
}

impl DetailedMetrics {
    pub fn total_score(&self) -> u8 {
        self.activity_score
            + self.community_score
            + self.responsiveness_score
            + self.maturity_score
            + self.documentation_score
    }
}

/// Health calculator for repositories
pub struct HealthCalculator;

impl HealthCalculator {
    /// Calculate health metrics for a repository
    pub fn calculate(
        stars: u32,
        forks: u32,
        watchers: u32,
        open_issues: u32,
        created_at: DateTime<Utc>,
        _updated_at: DateTime<Utc>,
        pushed_at: DateTime<Utc>,
        is_archived: bool,
        has_description: bool,
        topics_count: usize,
    ) -> HealthMetrics {
        let now = Utc::now();

        // If archived, score is 0
        if is_archived {
            return HealthMetrics {
                score: 0,
                status: HealthStatus::Critical,
                maintenance: MaintenanceLevel::Abandoned,
                metrics: DetailedMetrics {
                    activity_score: 0,
                    community_score: 0,
                    responsiveness_score: 0,
                    maturity_score: 0,
                    documentation_score: 0,
                },
            };
        }

        // Calculate individual scores
        let activity_score = Self::calculate_activity_score(pushed_at, now);
        let community_score = Self::calculate_community_score(stars, forks, watchers);
        let responsiveness_score = Self::calculate_responsiveness_score(open_issues, stars);
        let maturity_score = Self::calculate_maturity_score(created_at, now);
        let documentation_score =
            Self::calculate_documentation_score(has_description, topics_count);

        let metrics = DetailedMetrics {
            activity_score,
            community_score,
            responsiveness_score,
            maturity_score,
            documentation_score,
        };

        let score = metrics.total_score();
        let status = HealthStatus::from_score(score);
        let maintenance = MaintenanceLevel::from_last_push(pushed_at, now);

        HealthMetrics {
            score,
            status,
            maintenance,
            metrics,
        }
    }

    /// Activity score (0-30): Recent push activity
    fn calculate_activity_score(pushed_at: DateTime<Utc>, now: DateTime<Utc>) -> u8 {
        let days_since_push = (now - pushed_at).num_days();

        match days_since_push {
            0..=7 => 30,       // Within a week: excellent
            8..=30 => 25,      // Within a month: very good
            31..=90 => 20,     // Within 3 months: good
            91..=180 => 15,    // Within 6 months: moderate
            181..=365 => 10,   // Within a year: low
            366..=730 => 5,    // Within 2 years: very low
            _ => 0,            // Over 2 years: inactive
        }
    }

    /// Community score (0-25): Based on popularity metrics
    fn calculate_community_score(stars: u32, forks: u32, watchers: u32) -> u8 {
        // Calculate a weighted community score
        // Stars are most important, then forks, then watchers
        let stars_score = match stars {
            0..=9 => 0,
            10..=49 => 5,
            50..=199 => 10,
            200..=999 => 15,
            1000..=4999 => 20,
            _ => 25,
        };

        let forks_bonus = if forks > 10 { 2 } else { 0 };
        let watchers_bonus = if watchers > 10 { 2 } else { 0 };

        (stars_score + forks_bonus + watchers_bonus).min(25)
    }

    /// Responsiveness score (0-20): Issue management
    fn calculate_responsiveness_score(open_issues: u32, stars: u32) -> u8 {
        // If no stars, this metric doesn't apply well
        if stars < 10 {
            return 15; // Neutral score for small projects
        }

        // Calculate ratio of open issues to stars
        let issue_ratio = if stars > 0 {
            (open_issues as f32) / (stars as f32)
        } else {
            0.0
        };

        // Lower ratio is better (fewer issues per star)
        match issue_ratio {
            r if r < 0.01 => 20,    // Excellent: < 1%
            r if r < 0.05 => 17,    // Very good: < 5%
            r if r < 0.10 => 14,    // Good: < 10%
            r if r < 0.20 => 11,    // Moderate: < 20%
            r if r < 0.30 => 8,     // Fair: < 30%
            _ => 5,                 // Poor: >= 30%
        }
    }

    /// Maturity score (0-15): Repository age
    fn calculate_maturity_score(created_at: DateTime<Utc>, now: DateTime<Utc>) -> u8 {
        let days_old = (now - created_at).num_days();

        match days_old {
            0..=30 => 3,         // Brand new
            31..=90 => 5,        // Very young
            91..=180 => 8,       // Young
            181..=365 => 11,     // Established
            366..=730 => 13,     // Mature
            _ => 15,             // Very mature (2+ years)
        }
    }

    /// Documentation score (0-10): Presence of documentation
    fn calculate_documentation_score(has_description: bool, topics_count: usize) -> u8 {
        let mut score = 0;

        // Description present
        if has_description {
            score += 5;
        }

        // Topics/tags help with discovery
        score += match topics_count {
            0 => 0,
            1..=2 => 2,
            3..=5 => 3,
            _ => 5,
        };

        score.min(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_health_status_from_score() {
        assert_eq!(HealthStatus::from_score(100), HealthStatus::Healthy);
        assert_eq!(HealthStatus::from_score(80), HealthStatus::Healthy);
        assert_eq!(HealthStatus::from_score(70), HealthStatus::Moderate);
        assert_eq!(HealthStatus::from_score(50), HealthStatus::Warning);
        assert_eq!(HealthStatus::from_score(20), HealthStatus::Critical);
    }

    #[test]
    fn test_maintenance_level_from_last_push() {
        let now = Utc::now();

        assert_eq!(
            MaintenanceLevel::from_last_push(now - Duration::days(15), now),
            MaintenanceLevel::Active
        );
        assert_eq!(
            MaintenanceLevel::from_last_push(now - Duration::days(60), now),
            MaintenanceLevel::Maintained
        );
        assert_eq!(
            MaintenanceLevel::from_last_push(now - Duration::days(120), now),
            MaintenanceLevel::Stale
        );
        assert_eq!(
            MaintenanceLevel::from_last_push(now - Duration::days(270), now),
            MaintenanceLevel::Inactive
        );
        assert_eq!(
            MaintenanceLevel::from_last_push(now - Duration::days(400), now),
            MaintenanceLevel::Abandoned
        );
    }

    #[test]
    fn test_calculate_healthy_repo() {
        let now = Utc::now();
        let created = now - Duration::days(730); // 2 years old
        let pushed = now - Duration::days(7); // Pushed last week

        let health = HealthCalculator::calculate(
            1000,   // stars
            200,    // forks
            50,     // watchers
            10,     // open issues
            created,
            now,
            pushed,
            false,  // not archived
            true,   // has description
            5,      // topics
        );

        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.maintenance, MaintenanceLevel::Active);
        assert!(health.score >= 80);
    }

    #[test]
    fn test_calculate_archived_repo() {
        let now = Utc::now();
        let created = now - Duration::days(365);
        let pushed = now - Duration::days(30);

        let health = HealthCalculator::calculate(
            5000, 100, 50, 5, created, now, pushed,
            true, // archived
            true, 5,
        );

        assert_eq!(health.score, 0);
        assert_eq!(health.status, HealthStatus::Critical);
    }

    #[test]
    fn test_calculate_abandoned_repo() {
        let now = Utc::now();
        let created = now - Duration::days(1095); // 3 years old
        let pushed = now - Duration::days(500); // No push in >1 year

        let health = HealthCalculator::calculate(
            50, 5, 2, 10, created, now, pushed, false, true, 2,
        );

        assert_eq!(health.maintenance, MaintenanceLevel::Abandoned);
        assert!(health.score < 60);
    }
}

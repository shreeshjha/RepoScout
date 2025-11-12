// Sparkline rendering utilities
use chrono::{DateTime, Utc, Duration};

/// Generate a sparkline visualization using Unicode block characters
/// Characters: ▁ ▂ ▃ ▄ ▅ ▆ ▇ █
pub fn render_sparkline(data: &[f64]) -> String {
    if data.is_empty() {
        return String::new();
    }

    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = data.iter().cloned().fold(0.0f64, f64::max);

    if max == 0.0 {
        return "▁".repeat(data.len());
    }

    data.iter()
        .map(|&v| {
            let ratio = (v / max * 7.0).min(7.0).max(0.0);
            chars[ratio as usize]
        })
        .collect()
}

/// Generate activity sparkline based on repository age and recent activity
/// Shows activity trend over the repository's lifetime
pub fn generate_activity_sparkline(
    created_at: DateTime<Utc>,
    pushed_at: DateTime<Utc>,
    stars: u32,
) -> String {
    let now = Utc::now();
    let age_days = (now - created_at).num_days().max(1);
    let days_since_push = (now - pushed_at).num_days();

    // Divide repo lifetime into 12 periods
    let periods = 12;
    let days_per_period = age_days / periods;

    if days_per_period == 0 {
        // Very new repo, show recent activity only
        let activity = if days_since_push < 1 {
            vec![10.0; periods as usize]
        } else if days_since_push < 7 {
            vec![8.0; periods as usize]
        } else if days_since_push < 30 {
            vec![5.0; periods as usize]
        } else {
            vec![2.0; periods as usize]
        };
        return render_sparkline(&activity);
    }

    // Simulate activity trend based on:
    // - Repository age (newer repos might have higher activity)
    // - Stars (popular repos likely have sustained activity)
    // - Recent push (shows current activity level)

    let mut activity_data = Vec::new();
    let base_activity = (stars as f64 / age_days as f64 * 100.0).min(10.0).max(1.0);

    for i in 0..periods {
        let period_progress = i as f64 / periods as f64;

        // Most repos start with high activity and taper off
        let age_factor = if period_progress < 0.3 {
            // Early days - high activity
            1.0
        } else if period_progress < 0.7 {
            // Middle period - stable
            0.8
        } else {
            // Recent period - check if still active
            let recency_boost = if days_since_push < 30 {
                1.2 // Recently active
            } else if days_since_push < 90 {
                0.7 // Moderately active
            } else {
                0.4 // Less active
            };
            recency_boost
        };

        let value = base_activity * age_factor;
        activity_data.push(value);
    }

    render_sparkline(&activity_data)
}

/// Generate star velocity sparkline showing growth rate over time
pub fn generate_star_velocity_sparkline(
    created_at: DateTime<Utc>,
    stars: u32,
) -> String {
    let now = Utc::now();
    let age_weeks = (now - created_at).num_weeks().max(1);

    let periods = 12.min(age_weeks as usize);
    if periods == 0 {
        return "▁".to_string();
    }

    let avg_stars_per_week = stars as f64 / age_weeks as f64;

    // Simulate star accumulation pattern
    // Most repos have initial spike, then steady growth or plateau
    let mut velocity_data = Vec::new();

    for i in 0..periods {
        let period_progress = i as f64 / periods as f64;

        // Common patterns:
        // - Initial spike (launches, HN/Reddit)
        // - Gradual growth
        // - Recent activity boost

        let velocity = if period_progress < 0.2 {
            // Initial launch period
            avg_stars_per_week * 1.5
        } else if period_progress < 0.8 {
            // Steady growth
            avg_stars_per_week * (1.0 - period_progress * 0.3)
        } else {
            // Recent period - slight uptick if popular
            if stars > 1000 {
                avg_stars_per_week * 1.1
            } else {
                avg_stars_per_week * 0.7
            }
        };

        velocity_data.push(velocity);
    }

    render_sparkline(&velocity_data)
}

/// Generate issue/PR activity sparkline
pub fn generate_issue_activity_sparkline(
    open_issues: u32,
    stars: u32,
    created_at: DateTime<Utc>,
) -> String {
    let now = Utc::now();
    let age_months = (now - created_at).num_days() / 30;
    let age_months = age_months.max(1);

    let periods = 12.min(age_months as usize);

    // Issue activity correlates with popularity and community engagement
    let issue_rate = open_issues as f64 / age_months as f64;
    let engagement = (stars as f64 / 100.0).min(10.0).max(1.0);

    let mut activity_data = Vec::new();

    for i in 0..periods {
        let period_progress = i as f64 / periods as f64;

        // Issues tend to increase as project matures, then stabilize
        let activity = if period_progress < 0.3 {
            issue_rate * 0.5 * engagement
        } else if period_progress < 0.7 {
            issue_rate * 1.0 * engagement
        } else {
            issue_rate * 0.8 * engagement
        };

        activity_data.push(activity);
    }

    render_sparkline(&activity_data)
}

/// Generate a simple health trend sparkline
pub fn generate_health_trend_sparkline(health_score: u8) -> String {
    // Show health trend over time (simulated)
    let periods = 12;
    let current_health = health_score as f64;

    let mut trend_data = Vec::new();

    // Healthy repos tend to maintain or improve
    // Unhealthy repos show decline
    for i in 0..periods {
        let progress = i as f64 / periods as f64;

        let health = if current_health > 70.0 {
            // Healthy repo - slight improvement over time
            current_health * (0.85 + progress * 0.15)
        } else if current_health > 40.0 {
            // Moderate health - stable or slight decline
            current_health * (1.0 - progress * 0.1)
        } else {
            // Poor health - decline
            current_health * (1.2 - progress * 0.4)
        };

        trend_data.push(health);
    }

    render_sparkline(&trend_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline_rendering() {
        let data = vec![1.0, 2.0, 3.0, 5.0, 8.0, 5.0, 3.0, 2.0];
        let sparkline = render_sparkline(&data);
        assert_eq!(sparkline.len(), 8);
        assert!(sparkline.contains('█')); // Should have max char
    }

    #[test]
    fn test_empty_sparkline() {
        let data: Vec<f64> = vec![];
        let sparkline = render_sparkline(&data);
        assert_eq!(sparkline, "");
    }

    #[test]
    fn test_zero_data_sparkline() {
        let data = vec![0.0, 0.0, 0.0];
        let sparkline = render_sparkline(&data);
        assert_eq!(sparkline, "▁▁▁");
    }
}

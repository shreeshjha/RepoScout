// Retry logic with exponential backoff
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,     // Start with 1 second
            max_delay_ms: 30000,         // Max 30 seconds
            backoff_multiplier: 2.0,     // Double each time
        }
    }
}

/// Execute a function with retry logic
///
/// Uses exponential backoff: if a request fails, we wait progressively
/// longer before trying again. This is polite to APIs and helps when
/// there are temporary network issues.
pub async fn with_retry<F, Fut, T, E>(
    config: &RetryConfig,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    let mut delay_ms = config.initial_delay_ms;

    loop {
        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!("Request succeeded after {} retries", attempt);
                }
                return Ok(result);
            }
            Err(err) => {
                attempt += 1;

                if attempt > config.max_retries {
                    warn!("Request failed after {} attempts: {}", config.max_retries, err);
                    return Err(err);
                }

                // Check if this is a retryable error
                // For now, we retry all errors, but we could be smarter
                // (e.g., don't retry 404s, but do retry 500s and network errors)

                warn!("Request failed (attempt {}/{}): {}. Retrying in {}ms...",
                    attempt, config.max_retries, err, delay_ms);

                sleep(Duration::from_millis(delay_ms)).await;

                // Exponential backoff: double the delay each time, up to max
                delay_ms = ((delay_ms as f64) * config.backoff_multiplier) as u64;
                delay_ms = delay_ms.min(config.max_delay_ms);
            }
        }
    }
}

/// Check if an HTTP status code is retryable
pub fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    // Retry on:
    // - 5xx server errors (server is having issues)
    // - 429 too many requests (rate limited)
    // - 408 request timeout
    // - 503 service unavailable
    status.is_server_error()
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::REQUEST_TIMEOUT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_succeeds_immediately() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let config = RetryConfig::default();
        let call_count = AtomicU32::new(0);

        let result = with_retry(&config, || async {
            call_count.fetch_add(1, Ordering::SeqCst);
            Ok::<_, &str>(42)
        })
        .await;

        assert_eq!(result, Ok(42));
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_failures() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 10, // Fast for testing
            max_delay_ms: 100,
            backoff_multiplier: 2.0,
        };
        let call_count = AtomicU32::new(0);

        let result = with_retry(&config, || async {
            let count = call_count.fetch_add(1, Ordering::SeqCst) + 1;
            if count < 3 {
                Err("temporary failure")
            } else {
                Ok(42)
            }
        })
        .await;

        assert_eq!(result, Ok(42));
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_gives_up_after_max_attempts() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let config = RetryConfig {
            max_retries: 2,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            backoff_multiplier: 2.0,
        };
        let call_count = AtomicU32::new(0);

        let result = with_retry(&config, || async {
            call_count.fetch_add(1, Ordering::SeqCst);
            Err::<i32, _>("permanent failure")
        })
        .await;

        assert_eq!(result, Err("permanent failure"));
        assert_eq!(call_count.load(Ordering::SeqCst), 3); // Initial attempt + 2 retries
    }

    #[test]
    fn test_retryable_status_codes() {
        assert!(is_retryable_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR));
        assert!(is_retryable_status(reqwest::StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(reqwest::StatusCode::SERVICE_UNAVAILABLE));
        assert!(is_retryable_status(reqwest::StatusCode::TOO_MANY_REQUESTS));

        assert!(!is_retryable_status(reqwest::StatusCode::NOT_FOUND));
        assert!(!is_retryable_status(reqwest::StatusCode::BAD_REQUEST));
        assert!(!is_retryable_status(reqwest::StatusCode::UNAUTHORIZED));
    }
}

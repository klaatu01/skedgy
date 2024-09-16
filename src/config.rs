use std::time::Duration;

/// Configuration for the `Skedgy` scheduler.
#[derive(Debug, Clone)]
pub struct SkedgyConfig {
    pub look_ahead_duration: Duration,
}

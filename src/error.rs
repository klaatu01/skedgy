use std::error::Error;

/// Errors that can occur during scheduling or running tasks with the `Skedgy` scheduler.
#[derive(Debug)]
pub enum SkedgyError {
    /// Error sending a message to the scheduler.
    SendError,
    /// Error receiving a message from the scheduler.
    RecvError,
    /// An invalid cron expression was provided.
    InvalidCron,
    /// An error occurred during a scheduler tick.
    TickError,
}

impl Error for SkedgyError {}

impl std::fmt::Display for SkedgyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkedgyError::SendError => write!(f, "Error sending message to scheduler"),
            SkedgyError::RecvError => write!(f, "Error receiving message from scheduler"),
            SkedgyError::InvalidCron => write!(f, "Invalid cron expression"),
            SkedgyError::TickError => write!(f, "Error during scheduler tick"),
        }
    }
}

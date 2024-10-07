//! # Skedgy - Asynchronous Task Scheduler
//!
//! Skedgy is a lightweight, asynchronous task scheduler written in Rust. It allows you to schedule tasks to run at specific times, after certain delays, or based on cron expressions. Skedgy is built using `tokio` for asynchronous execution and is designed to be efficient and easy to use.
//!
//! ## Features
//!
//! - **Run tasks at a specific time**: Schedule tasks to run at any `DateTime<Utc>`.
//! - **Run tasks after a delay**: Schedule tasks to run after a specified `Duration`.
//! - **Cron scheduling**: Schedule tasks using cron expressions for recurring tasks.
//!
//! ## Installation
//!
//! ```bash
//! cargo add skedgy
//! ```
//!
//! The test suite includes tests for scheduling tasks at specific times, after delays, and with cron expressions. It also checks error handling for invalid cron expressions.
//!
//! ## Contributing
//!
//! Contributions are welcome! Please feel free to submit a pull request or open an issue.
//!
//! ## License
//!
//! This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
mod command;
mod config;
mod context;
mod error;
mod handler;
mod scheduler;
mod skedgy;
mod task;

pub use config::SkedgyConfig;
pub use context::SkedgyContext;
pub use error::SkedgyError;
pub use handler::{Metadata, SkedgyHandler};
pub use skedgy::Skedgy;

#[cfg(test)]
mod tests {
    use self::handler::Metadata;
    use super::*;
    use crate::SkedgyHandler;
    use chrono::Utc;
    use futures::lock::Mutex;
    use std::sync::Arc;
    use std::time::Duration;

    #[derive(Clone)]
    struct MockHandler {
        counter: Arc<Mutex<i32>>,
        done_tx: Option<async_channel::Sender<()>>,
    }

    impl MockHandler {
        fn new(counter: Arc<Mutex<i32>>, done_tx: Option<async_channel::Sender<()>>) -> Self {
            MockHandler { counter, done_tx }
        }
    }

    #[derive(Clone)]
    struct MockHandler2 {
        counter: Arc<Mutex<i32>>,
        done_tx: Option<async_channel::Sender<()>>,
    }

    impl SkedgyHandler for MockHandler2 {
        type Context = MockContext;
        async fn handle(&self, _ctx: &Self::Context, _metadata: Metadata) {
            let mut count = self.counter.lock().await;
            *count += 1;
            if let Some(tx) = &self.done_tx {
                tx.send(()).await.expect("Failed to send done signal");
            }
        }
    }

    #[derive(Clone)]
    struct MockContext {}

    impl SkedgyHandler for MockHandler {
        type Context = MockContext;
        async fn handle(&self, _ctx: &Self::Context, _metadata: Metadata) {
            let mut count = self.counter.lock().await;
            *count += 1;
            if let Some(tx) = &self.done_tx {
                tx.send(()).await.expect("Failed to send done signal");
            }
        }
    }

    fn create_scheduler<Ctx: SkedgyContext>(tick_interval: Duration, ctx: Ctx) -> Skedgy<Ctx> {
        let config = SkedgyConfig {
            look_ahead_duration: tick_interval,
        };
        Skedgy::new(config, ctx)
    }

    #[tokio::test]
    async fn test_run_at() {
        let counter = Arc::new(Mutex::new(0));
        let (tx, rx) = async_channel::bounded(1);
        let handler = MockHandler::new(counter.clone(), Some(tx));

        let skedgy = create_scheduler(Duration::from_millis(100), MockContext {});
        let run_at = Utc::now() + Duration::from_millis(200);

        skedgy
            .named("test_task")
            .datetime(run_at)
            .task(handler)
            .await
            .expect("Failed to schedule task");

        rx.recv().await.expect("Failed to receive done signal");
        assert_eq!(*counter.lock().await, 1);
    }

    #[tokio::test]
    async fn test_run_in() {
        let counter = Arc::new(Mutex::new(0));
        let (tx, rx) = async_channel::bounded(1);
        let handler = MockHandler::new(counter.clone(), Some(tx));

        let skedgy = create_scheduler(Duration::from_millis(100), MockContext {});

        skedgy
            .named("test_task")
            .duration(Duration::from_millis(200))
            .task(handler)
            .await
            .expect("Failed to schedule task");

        rx.recv().await.expect("Failed to receive done signal");
        assert_eq!(*counter.lock().await, 1);
    }

    #[tokio::test]
    async fn test_cron() {
        let counter = Arc::new(Mutex::new(0));
        let (tx, rx) = async_channel::bounded(1);
        let handler = MockHandler::new(counter.clone(), Some(tx));

        let skedgy = create_scheduler(Duration::from_millis(100), MockContext {});

        skedgy
            .named("test_task")
            .cron("0/1 * * * * * *")
            .task(handler)
            .await
            .expect("Failed to schedule cron task");

        rx.recv().await.expect("Failed to receive done signal");
        assert_eq!(*counter.lock().await, 1);
    }

    #[tokio::test]
    async fn test_multiple_schedules() {
        let counter = Arc::new(Mutex::new(0));
        let (tx1, rx1) = async_channel::bounded(1);
        let handler1 = MockHandler::new(counter.clone(), Some(tx1));

        let (tx2, rx2) = async_channel::bounded(1);
        let handler2 = MockHandler::new(counter.clone(), Some(tx2));

        let skedgy = create_scheduler(Duration::from_millis(100), MockContext {});

        let run_at = Utc::now() + Duration::from_millis(200);

        skedgy
            .named("task1")
            .datetime(run_at)
            .task(handler1)
            .await
            .expect("Failed to schedule task");

        skedgy
            .named("task2")
            .duration(Duration::from_millis(400))
            .task(handler2)
            .await
            .expect("Failed to schedule task");

        rx1.recv()
            .await
            .expect("Failed to receive done signal for first task");
        assert_eq!(*counter.lock().await, 1);

        rx2.recv()
            .await
            .expect("Failed to receive done signal for second task");
        assert_eq!(*counter.lock().await, 2);
    }

    #[tokio::test]
    async fn test_remove_task() {
        let counter = Arc::new(Mutex::new(0));
        let handler = MockHandler::new(counter.clone(), None);

        let skedgy = create_scheduler(Duration::from_millis(100), MockContext {});
        let run_at = Utc::now() + Duration::from_millis(200);

        skedgy
            .named("remove_task")
            .datetime(run_at)
            .task(handler)
            .await
            .expect("Failed to schedule task");

        skedgy
            .remove("remove_task")
            .await
            .expect("Failed to remove task");

        tokio::time::sleep(Duration::from_millis(300)).await;

        assert_eq!(*counter.lock().await, 0);
    }

    #[tokio::test]
    async fn test_remove_cron_task() {
        env_logger::init();
        let counter = Arc::new(Mutex::new(0));
        let handler = MockHandler::new(counter.clone(), None);

        let skedgy = create_scheduler(Duration::from_millis(10), MockContext {});

        skedgy
            .named("cron_task")
            .cron("0/1 * * * * * *")
            .task(handler)
            .await
            .expect("Failed to schedule cron task");

        tokio::time::sleep(Duration::from_millis(1000)).await;

        skedgy
            .remove("cron_task")
            .await
            .expect("Failed to remove cron task");

        tokio::time::sleep(Duration::from_millis(1500)).await;

        assert_eq!(*counter.lock().await, 1);
    }
}

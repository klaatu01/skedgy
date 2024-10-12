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
mod dep;
mod error;
mod scheduler;
mod skedgy;
mod task;

pub use config::SkedgyConfig;
pub use dep::{Dep, DependencyStore};
pub use error::SkedgyError;
pub use skedgy::Skedgy;
pub use skedgy_derive::task;
pub use task::{BoxFuture, Task};

#[task]
async fn test() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use futures::lock::Mutex;
    use skedgy_derive::task;
    use std::sync::Arc;
    use std::time::Duration;

    #[task]
    async fn mock_handler(counter: Arc<Mutex<i32>>, done_tx: Option<async_channel::Sender<()>>) {
        let mut count = counter.lock().await;
        *count += 1;
        if let Some(tx) = done_tx {
            tx.send(()).await.expect("Failed to send done signal");
        }
    }

    #[task]
    async fn handler_with_deps(
        counter: Arc<Mutex<i32>>,
        dep: Dep<i32>,
        done_tx: Option<async_channel::Sender<()>>,
    ) {
        let mut count = counter.lock().await;
        *count += *dep.inner();
        if let Some(tx) = done_tx {
            tx.send(()).await.expect("Failed to send done signal");
        }
    }

    fn create_scheduler(tick_interval: Duration) -> Skedgy {
        let config = SkedgyConfig {
            look_ahead_duration: tick_interval,
        };
        let mut dep_store = DependencyStore::new();
        dep_store.insert::<i32>(10);

        Skedgy::new(config, dep_store)
    }

    #[tokio::test]
    async fn test_run_at() {
        let counter = Arc::new(Mutex::new(0));
        let (tx, rx) = async_channel::bounded(1);
        let handler = mock_handler::new(counter.clone(), Some(tx));

        let skedgy = create_scheduler(Duration::from_millis(100));
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
        let handler = mock_handler::new(counter.clone(), Some(tx));

        let skedgy = create_scheduler(Duration::from_millis(100));

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
        let handler = mock_handler::new(counter.clone(), Some(tx));

        let skedgy = create_scheduler(Duration::from_millis(100));

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
        let handler1 = mock_handler::new(counter.clone(), Some(tx1));

        let (tx2, rx2) = async_channel::bounded(1);
        let handler2 = mock_handler::new(counter.clone(), Some(tx2));

        let skedgy = create_scheduler(Duration::from_millis(100));

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
        let handler = mock_handler::new(counter.clone(), None);

        let skedgy = create_scheduler(Duration::from_millis(100));
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
        let handler = mock_handler::new(counter.clone(), None);

        let skedgy = create_scheduler(Duration::from_millis(10));

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

    #[tokio::test]
    async fn test_dependency_injection() {
        let counter = Arc::new(Mutex::new(0));
        let (tx, rx) = async_channel::bounded(1);
        let handler = handler_with_deps::new(counter.clone(), Some(tx));

        let skedgy = create_scheduler(Duration::from_millis(100));
        let run_at = Utc::now() + Duration::from_millis(200);

        skedgy
            .named("test_task")
            .datetime(run_at)
            .task(handler)
            .await
            .expect("Failed to schedule task");

        rx.recv().await.expect("Failed to receive done signal");
        assert_eq!(*counter.lock().await, 10);
    }
}

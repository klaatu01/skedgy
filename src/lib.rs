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
//! ## Usage
//!
//! ### 1. Define Your Task Handler
//!
//! Implement the `SkedgyHandler` trait for your task. This trait requires the `handle` method, which is an asynchronous function that defines the task's behavior.
//!
//! ```rust
//! use skedgy::SkedgyHandler;
//!
//! #[derive(Clone)]
//! struct MyTask;
//!
//! impl SkedgyHandler for MyTask {
//!     async fn handle(&self) {
//!         println!("Task is running!");
//!     }
//! }
//! ```
//!
//! ### 2. Create a Scheduler
//!
//! Initialize a scheduler with a tick interval:
//!
//! ```rust
//! use skedgy::{Skedgy, SkedgyConfig};
//! use std::time::Duration;
//!
//! let config = SkedgyConfig {
//!     tick_interval: Duration::from_millis(100),
//! };
//! let mut scheduler = Skedgy::new(config).expect("Failed to create scheduler");
//! ```
//!
//! ### 3. Schedule Tasks
//!
//! You can schedule tasks to run at specific times, after a delay, or using cron expressions.
//!
//! ```rust
//! use chrono::Utc;
//!
//! // Schedule a task to run at a specific time
//! scheduler.run_at(Utc::now() + chrono::Duration::seconds(10), MyTask).await.expect("Failed to schedule task");
//!
//! // Schedule a task to run after a delay
//! scheduler.run_in(Duration::from_secs(5), MyTask).await.expect("Failed to schedule task");
//!
//! // Schedule a task using a cron expression
//! scheduler.cron("0/1 * * * * * *", MyTask).await.expect("Failed to schedule cron task");
//! ```
//!
//! ### 4. Run the Scheduler
//!
//! The scheduler is automatically run when it is created and will continue to process tasks until the program ends.
//!
//! ## Running Tests
//!
//! To run the tests, use the following command:
//!
//! ```sh
//! cargo test
//! ```
//!
//! Make sure to include the necessary dependencies in your `Cargo.toml`:
//!
//! ```toml
//! [dev-dependencies]
//! tokio = { version = "1", features = ["full", "macros"] }
//! async-trait = "0.1"
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

use std::{collections::BTreeMap, error::Error, str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use nanoid::nanoid;

/// A trait for defining task handlers that can be scheduled by the `Skedgy` scheduler.
/// Implement this trait for your task handler and define the task's behavior in the `handle` method.
pub trait SkedgyHandler: Clone + Send + 'static {
    fn handle(&self) -> impl std::future::Future<Output = ()> + Send;
}

/// The main scheduler struct that allows you to schedule tasks to run at specific times, after delays, or using cron expressions.
/// Create a new `Skedgy` instance using the `new` method and schedule tasks using the `run_at`, `run_in`, and `cron` methods.
pub struct Skedgy<T: SkedgyHandler> {
    tx: async_channel::Sender<SkedgyCommand<T>>,
}

impl<T: SkedgyHandler> Skedgy<T> {
    pub fn new(config: SkedgyConfig) -> Self {
        let (tx, rx) = async_channel::unbounded();
        let mut scheduler = SchedulerLoop {
            config,
            schedules: BTreeMap::new(),
            crons: BTreeMap::new(),
            rx,
        };
        tokio::spawn(async move {
            if let Err(e) = scheduler.run().await {
                log::error!("Scheduler encountered an error: {}", e);
            }
        });
        Self { tx }
    }

    /// Schedule a task to run at a specific `DateTime<Utc>`.
    /// The `handler` parameter should be a struct that implements the `SkedgyHandler` trait.
    pub async fn run_at(&mut self, datetime: DateTime<Utc>, handler: T) -> Result<(), SkedgyError> {
        self.tx
            .send(SkedgyCommand::AddAt(datetime, handler.into()))
            .await
            .map_err(|_| SkedgyError::SendError)?;
        Ok(())
    }

    /// Schedule a task to run after a specified `Duration`.
    /// The `handler` parameter should be a struct that implements the `SkedgyHandler` trait.
    pub async fn run_in(&mut self, duration: Duration, handler: T) -> Result<(), SkedgyError> {
        let datetime = Utc::now() + duration;
        self.run_at(datetime, handler).await
    }

    /// Schedule a task using a cron expression.
    /// The `handler` parameter should be a struct that implements the `SkedgyHandler` trait.
    pub async fn cron(&mut self, cron: &str, handler: T) -> Result<(), SkedgyError> {
        cron::Schedule::from_str(cron).map_err(|_| SkedgyError::InvalidCron)?;
        self.tx
            .send(SkedgyCommand::AddCron(cron.to_string(), handler.into()))
            .await
            .map_err(|_| SkedgyError::SendError)?;
        Ok(())
    }
}

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

struct SchedulerLoop<T: SkedgyHandler> {
    config: SkedgyConfig,
    schedules: BTreeMap<DateTime<Utc>, Vec<SkedgyTask<T>>>,
    crons: BTreeMap<String, Vec<SkedgyTask<T>>>,
    rx: async_channel::Receiver<SkedgyCommand<T>>,
}

impl<T: SkedgyHandler> SchedulerLoop<T> {
    fn insert(&mut self, datetime: DateTime<Utc>, task: SkedgyTask<T>) {
        self.schedules.entry(datetime).or_default().push(task);
    }

    fn insert_cron(&mut self, cron: String, task: SkedgyTask<T>) {
        self.crons.entry(cron).or_default().push(task);
    }

    fn query(&mut self, end: DateTime<Utc>) -> Vec<SkedgyTask<T>> {
        let after = self.schedules.split_off(&end);
        let before = self
            .schedules
            .iter()
            .flat_map(|(_, v)| v)
            .cloned()
            .collect();
        self.schedules = after;
        before
    }

    fn query_crons(&mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<SkedgyTask<T>> {
        let mut tasks = Vec::new();
        for (cron, cron_tasks) in self.crons.iter() {
            let schedule = match cron::Schedule::from_str(cron) {
                Ok(schedule) => schedule,
                Err(_) => continue,
            };

            let upcoming_runs = schedule.after(&start);
            for run_time in upcoming_runs {
                if run_time > end {
                    break;
                }

                for task in cron_tasks.iter() {
                    tasks.push(task.clone());
                }
            }
        }
        tasks
    }

    async fn drain_channel(&mut self) -> Result<Vec<SkedgyCommand<T>>, SkedgyError> {
        let mut commands = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(command) => commands.push(command),
                Err(e) => {
                    if e.is_empty() {
                        return Ok(commands);
                    } else {
                        return Err(SkedgyError::RecvError);
                    }
                }
            }
        }
    }

    fn handle_schedules(&self, tasks: Vec<SkedgyTask<T>>) {
        tokio::spawn(async move {
            for task in tasks {
                task.handler.handle().await;
            }
        });
    }

    fn handle_commands(&mut self, commands: Vec<SkedgyCommand<T>>) {
        for command in commands {
            self.handle_command(command);
        }
    }

    fn handle_command(&mut self, command: SkedgyCommand<T>) {
        match command {
            SkedgyCommand::AddAt(datetime, handler) => self.insert(datetime, handler),
            SkedgyCommand::AddCron(cron, handler) => self.insert_cron(cron, handler),
        }
    }

    async fn run(&mut self) -> Result<(), SkedgyError> {
        let mut last_tick_start = chrono::Utc::now();
        loop {
            log::debug!("Scheduler tick");
            let tick_start = chrono::Utc::now();

            let commands = self.drain_channel().await?;
            self.handle_commands(commands);

            let tasks = self.query(tick_start);
            self.handle_schedules(tasks);

            let crons = self.query_crons(last_tick_start, tick_start);
            self.handle_schedules(crons);

            let tick_end = chrono::Utc::now();
            let tick_duration = tick_end
                .signed_duration_since(tick_start)
                .to_std()
                .map_err(|_| SkedgyError::TickError)?;

            if tick_duration < self.config.tick_interval {
                log::debug!(
                    "Sleeping for {:?}",
                    self.config.tick_interval - tick_duration
                );
                tokio::time::sleep(self.config.tick_interval - tick_duration).await;
            } else {
                log::warn!("Scheduler tick took longer than the tick interval");
            }

            last_tick_start = tick_start;
            log::debug!("Scheduler tick took {:?}", tick_duration);
        }
    }
}

/// Configuration for the `Skedgy` scheduler.
#[derive(Debug, Clone)]
pub struct SkedgyConfig {
    pub tick_interval: Duration,
}

#[derive(Debug, Clone)]
struct SkedgyTask<T: SkedgyHandler> {
    #[allow(dead_code)]
    pub id: String,
    pub handler: T,
}

impl<T: SkedgyHandler> SkedgyTask<T> {
    pub fn new(handler: T) -> Self {
        Self {
            handler,
            id: nanoid!(10),
        }
    }
}

impl<T: SkedgyHandler> From<T> for SkedgyTask<T> {
    fn from(handler: T) -> Self {
        Self::new(handler)
    }
}

enum SkedgyCommand<T: SkedgyHandler> {
    AddAt(DateTime<Utc>, SkedgyTask<T>),
    AddCron(String, SkedgyTask<T>),
}

#[cfg(test)]
mod tests {
    use futures::lock::Mutex;

    use super::*;
    use std::sync::Arc;

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

    impl SkedgyHandler for MockHandler {
        async fn handle(&self) {
            let mut count = self.counter.lock().await;
            *count += 1;
            if let Some(tx) = &self.done_tx {
                tx.send(()).await.expect("Failed to send done signal");
            }
        }
    }

    fn create_scheduler<T: SkedgyHandler>(tick_interval: Duration) -> Skedgy<T> {
        let config = SkedgyConfig { tick_interval };
        Skedgy::new(config)
    }

    #[tokio::test]
    async fn test_run_at() {
        let counter = Arc::new(Mutex::new(0));
        let (tx, rx) = async_channel::bounded(1);
        let handler = MockHandler::new(counter.clone(), Some(tx));

        let mut scheduler = create_scheduler::<MockHandler>(Duration::from_millis(100));
        let run_at = Utc::now() + Duration::from_millis(200);
        scheduler
            .run_at(run_at, handler)
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

        let mut scheduler = create_scheduler::<MockHandler>(Duration::from_millis(100));
        scheduler
            .run_in(Duration::from_millis(200), handler)
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

        let mut scheduler = create_scheduler::<MockHandler>(Duration::from_millis(100));
        scheduler
            .cron("0/1 * * * * * *", handler)
            .await
            .expect("Failed to schedule cron task");

        rx.recv().await.expect("Failed to receive done signal");
        // Since cron runs every second, the first run will immediately trigger.
        assert_eq!(*counter.lock().await, 1);
    }

    #[tokio::test]
    async fn test_multiple_schedules() {
        let counter = Arc::new(Mutex::new(0));
        let (tx1, rx1) = async_channel::bounded(1);
        let handler1 = MockHandler::new(counter.clone(), Some(tx1));

        let (tx2, rx2) = async_channel::bounded(1);
        let handler2 = MockHandler::new(counter.clone(), Some(tx2));

        let mut scheduler = create_scheduler::<MockHandler>(Duration::from_millis(100));

        let run_at = Utc::now() + Duration::from_millis(200);
        scheduler
            .run_at(run_at, handler1)
            .await
            .expect("Failed to schedule task");

        scheduler
            .run_in(Duration::from_millis(400), handler2)
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
    async fn test_invalid_cron() {
        let counter = Arc::new(Mutex::new(0));
        let handler = MockHandler::new(counter.clone(), None);

        let mut scheduler = create_scheduler::<MockHandler>(Duration::from_millis(100));
        let result = scheduler.cron("invalid_cron_expression", handler).await;

        assert!(result.is_err());
        if let Err(SkedgyError::InvalidCron) = result {
            // Expected error
        } else {
            panic!("Expected InvalidCron error");
        }
    }
}

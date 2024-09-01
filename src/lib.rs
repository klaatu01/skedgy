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

use std::{collections::BTreeMap, error::Error, str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use cron::Schedule;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

/// A trait for defining task handlers that can be scheduled by the `Skedgy` scheduler.
/// Implement this trait for your task handler and define the task's behavior in the `handle` method.
pub trait SkedgyHandler: Clone + Send + 'static {
    fn handle(&self) -> impl std::future::Future<Output = ()> + Send;
}

/// The main scheduler struct that allows you to schedule tasks to run at specific times, after delays, or using cron expressions.
/// Create a new `Skedgy` instance using the `new` method and schedule tasks using the `run_at`, `run_in`, and `cron` methods.
pub struct Skedgy<T: SkedgyHandler> {
    tx: async_channel::Sender<SkedgyCommand<T>>,
    terminate_tx: async_channel::Sender<async_channel::Sender<()>>,
}

fn serialize_datetime<S>(datetime: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&datetime.to_rfc3339())
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s)
        .map_err(serde::de::Error::custom)
        .map(|dt| dt.with_timezone(&Utc))
}

fn serialize_schedule<S>(schedule: &Schedule, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&schedule.to_string())
}

fn deserialize_schedule<'de, D>(deserializer: D) -> Result<Schedule, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Schedule::from_str(&s).map_err(serde::de::Error::custom)
}

#[derive(Clone, Serialize, Deserialize)]
pub enum TaskKind {
    #[serde(
        serialize_with = "serialize_datetime",
        deserialize_with = "deserialize_datetime"
    )]
    At(DateTime<Utc>),

    In(Duration),

    #[serde(
        serialize_with = "serialize_schedule",
        deserialize_with = "deserialize_schedule"
    )]
    Cron(Schedule),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SkedgyTask<T: SkedgyHandler> {
    id: String,
    kind: TaskKind,
    handler: T,
}

impl<T: SkedgyHandler> SkedgyTask<T> {
    pub fn named(id: &str) -> SkedgyTaskBuilder<T> {
        SkedgyTaskBuilder::named(id)
    }

    pub fn anonymous() -> SkedgyTaskBuilder<T> {
        SkedgyTaskBuilder::new()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SkedgyState<T: SkedgyHandler> {
    tasks: Vec<SkedgyTask<T>>,
}

impl<T: SkedgyHandler> SkedgyState<T> {
    pub fn new(
        crons: Vec<(Schedule, SkedgyTask<T>)>,
        schedules: BTreeMap<DateTime<Utc>, Vec<SkedgyTask<T>>>,
    ) -> Self {
        Self {
            tasks: crons
                .into_iter()
                .map(|(_, task)| task)
                .chain(schedules.values().flat_map(|v| v.iter().cloned()))
                .collect(),
        }
    }

    pub fn crons(&self) -> Vec<(Schedule, SkedgyTask<T>)> {
        self.tasks
            .iter()
            .filter_map(|task| match &task.kind {
                TaskKind::Cron(schedule) => Some((schedule.clone(), task.clone())),
                _ => None,
            })
            .collect()
    }

    pub fn schedules(&self) -> BTreeMap<DateTime<Utc>, Vec<SkedgyTask<T>>> {
        let mut schedules: BTreeMap<DateTime<Utc>, Vec<SkedgyTask<T>>> = BTreeMap::new();
        for task in &self.tasks {
            if let TaskKind::At(datetime) = &task.kind {
                schedules.entry(*datetime).or_default().push(task.clone());
            }
        }
        schedules
    }
}

pub struct SkedgyTaskBuilder<T: SkedgyHandler> {
    kind: Option<TaskKind>,
    handler: Option<T>,
    id: Option<String>,
}

impl<T: SkedgyHandler> SkedgyTaskBuilder<T> {
    pub fn named(id: &str) -> Self {
        Self {
            kind: None,
            handler: None,
            id: Some(id.to_string()),
        }
    }

    pub fn new() -> Self {
        Self {
            kind: None,
            handler: None,
            id: nanoid!(10).into(),
        }
    }

    pub fn at(&mut self, datetime: DateTime<Utc>) -> &mut Self {
        self.kind = Some(TaskKind::At(datetime));
        self
    }

    pub fn r#in(&mut self, duration: Duration) -> &mut Self {
        let datetime = Utc::now() + duration;
        self.kind = Some(TaskKind::At(datetime));
        self
    }

    pub fn cron(&mut self, pattern: &str) -> Result<&mut Self, SkedgyError> {
        let schedule = cron::Schedule::from_str(pattern).map_err(|_| SkedgyError::InvalidCron)?;
        self.kind = Some(TaskKind::Cron(schedule));
        Ok(self)
    }

    pub fn handler(&mut self, handler: T) -> &mut Self {
        self.handler = Some(handler);
        self
    }

    pub fn build(&self) -> Result<SkedgyTask<T>, SkedgyError> {
        let kind = self.kind.clone().ok_or(SkedgyError::InvalidCron)?;
        let handler = self.handler.clone().ok_or(SkedgyError::InvalidCron)?;
        let id = self.id.clone().unwrap_or_else(|| nanoid!(10));
        Ok(SkedgyTask { id, kind, handler })
    }
}

impl<T: SkedgyHandler> Skedgy<T> {
    pub fn new(config: SkedgyConfig) -> Self {
        let (tx, rx) = async_channel::unbounded();
        let (terminate_tx, terminate_rx) = async_channel::bounded(1);
        let mut scheduler = SkedgyScheduler::new(config, rx, terminate_rx);
        tokio::spawn(async move {
            if let Err(e) = scheduler.run().await {
                log::error!("Scheduler encountered an error: {}", e);
            }
        });
        Self { tx, terminate_tx }
    }

    /// Schedule a task to run at a specific `DateTime<Utc>`.
    /// The `handler` parameter should be a struct that implements the `SkedgyHandler` trait.
    pub async fn schedule(&self, task: SkedgyTask<T>) -> Result<(), SkedgyError> {
        self.tx
            .send(SkedgyCommand::Add(task))
            .await
            .map_err(|_| SkedgyError::SendError)?;
        Ok(())
    }

    /// Remove a cron task by its ID.
    /// The `id` parameter should be the ID returned by the `cron` method.
    pub async fn remove(&self, id: &str) -> Result<(), SkedgyError> {
        self.tx
            .send(SkedgyCommand::Remove(id.to_string()))
            .await
            .map_err(|_| SkedgyError::SendError)?;
        Ok(())
    }

    /// Update an existing task with a new schedule.
    /// The `handler` parameter should be a struct that implements the `SkedgyHandler` trait.
    pub async fn update(&self, task: SkedgyTask<T>) -> Result<(), SkedgyError> {
        self.tx
            .send(SkedgyCommand::Update(task))
            .await
            .map_err(|_| SkedgyError::SendError)?;
        Ok(())
    }

    /// Get the current state of the scheduler, including all scheduled tasks.
    pub async fn state(&self) -> Result<SkedgyState<T>, SkedgyError> {
        let (tx, rx) = async_channel::bounded(1);
        self.tx
            .send(SkedgyCommand::GetState(tx))
            .await
            .map_err(|_| SkedgyError::SendError)?;
        rx.recv().await.map_err(|_| SkedgyError::RecvError)
    }

    /// Load a previous state into the scheduler.
    pub async fn load(&self, state: SkedgyState<T>) -> Result<(), SkedgyError> {
        self.tx
            .send(SkedgyCommand::LoadState(state))
            .await
            .map_err(|_| SkedgyError::SendError)?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), SkedgyError> {
        let (tx, rx) = async_channel::bounded(1);
        self.terminate_tx
            .send(tx)
            .await
            .map_err(|_| SkedgyError::SendError)?;
        rx.recv().await.map_err(|_| SkedgyError::RecvError)?;
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

struct SkedgyScheduler<T: SkedgyHandler> {
    config: SkedgyConfig,
    schedules: BTreeMap<DateTime<Utc>, Vec<SkedgyTask<T>>>,
    crons: Vec<(cron::Schedule, SkedgyTask<T>)>,
    rx: async_channel::Receiver<SkedgyCommand<T>>,
    terminate_rx: async_channel::Receiver<async_channel::Sender<()>>,
}

impl<T: SkedgyHandler> SkedgyScheduler<T> {
    fn new(
        config: SkedgyConfig,
        rx: async_channel::Receiver<SkedgyCommand<T>>,
        terminate_rx: async_channel::Receiver<async_channel::Sender<()>>,
    ) -> Self {
        Self {
            config,
            schedules: BTreeMap::new(),
            crons: Vec::new(),
            rx,
            terminate_rx,
        }
    }

    fn insert(&mut self, datetime: DateTime<Utc>, task: SkedgyTask<T>) {
        self.schedules.entry(datetime).or_default().push(task);
    }

    fn insert_cron(&mut self, schedule: Schedule, task: SkedgyTask<T>) {
        self.crons.push((schedule, task));
    }

    fn remove_cron(&mut self, id: String) {
        self.crons.retain(|(_, task)| task.id != id);
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
        self.crons
            .iter()
            .filter_map(|(schedule, task)| {
                let upcoming_runs = schedule.after(&start);
                let runs: Vec<_> = upcoming_runs
                    .take_while(|run_time| run_time < &end)
                    .collect();
                if runs.is_empty() {
                    None
                } else {
                    Some(task.clone())
                }
            })
            .collect()
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
            futures::future::join_all(tasks.iter().map(|task| task.handler.handle())).await;
        });
    }

    async fn handle_commands(&mut self, commands: Vec<SkedgyCommand<T>>) {
        for command in commands {
            self.handle_command(command).await;
        }
    }

    fn state(&mut self) -> SkedgyState<T> {
        SkedgyState::new(self.crons.clone(), self.schedules.clone())
    }

    fn load(&mut self, state: SkedgyState<T>) {
        self.crons = state.crons();
        self.schedules = state.schedules();
    }

    async fn handle_command(&mut self, command: SkedgyCommand<T>) {
        match command {
            SkedgyCommand::Add(task) => match task.kind {
                TaskKind::At(datetime) => self.insert(datetime, task),
                TaskKind::In(duration) => {
                    let datetime = Utc::now() + duration;
                    self.insert(datetime, task);
                }
                TaskKind::Cron(ref schedule) => self.insert_cron(schedule.clone(), task),
            },
            SkedgyCommand::Remove(id) => {
                self.schedules.values_mut().for_each(|v| {
                    v.retain(|t| t.id != id);
                });
                self.remove_cron(id);
            }
            SkedgyCommand::Update(task) => match task.kind {
                TaskKind::At(datetime) => self.insert(datetime, task),
                TaskKind::In(duration) => {
                    let datetime = Utc::now() + duration;
                    self.insert(datetime, task);
                }
                TaskKind::Cron(ref schedule) => {
                    self.remove_cron(task.id.clone());
                    self.insert_cron(schedule.clone(), task);
                }
            },
            SkedgyCommand::GetState(tx) => {
                let state = self.state();
                let _ = tx.send(state).await;
            }
            SkedgyCommand::LoadState(state) => {
                self.load(state);
            }
        }
    }

    async fn run(&mut self) -> Result<(), SkedgyError> {
        let mut last_tick_start = chrono::Utc::now();
        loop {
            if let Ok(tx) = self.terminate_rx.try_recv() {
                let _ = tx.send(()).await;
                log::info!("Terminating scheduler");
                return Ok(());
            }

            log::debug!("Scheduler tick");
            let tick_start = chrono::Utc::now();

            let commands = self.drain_channel().await?;
            log::debug!("Received {} commands", commands.len());
            self.handle_commands(commands).await;

            let tasks = self.query(tick_start);
            log::debug!("Running {} tasks", tasks.len());
            self.handle_schedules(tasks);

            let crons = self.query_crons(last_tick_start, tick_start);
            log::debug!("Running {} cron tasks", crons.len());
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

enum SkedgyCommand<T: SkedgyHandler> {
    Add(SkedgyTask<T>),
    Remove(String),
    Update(SkedgyTask<T>),
    GetState(async_channel::Sender<SkedgyState<T>>),
    LoadState(SkedgyState<T>),
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

        let scheduler = create_scheduler::<MockHandler>(Duration::from_millis(100));
        let run_at = Utc::now() + Duration::from_millis(200);
        let task = SkedgyTaskBuilder::named("test_task")
            .at(run_at)
            .handler(handler)
            .build()
            .expect("Failed to build task");

        scheduler
            .schedule(task)
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

        let scheduler = create_scheduler::<MockHandler>(Duration::from_millis(100));
        let task = SkedgyTaskBuilder::named("test_task")
            .r#in(Duration::from_millis(200))
            .handler(handler)
            .build()
            .expect("Failed to build task");
        scheduler
            .schedule(task)
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

        let scheduler = create_scheduler::<MockHandler>(Duration::from_millis(100));
        let task = SkedgyTaskBuilder::named("test_task")
            .cron("0/1 * * * * * *")
            .expect("Failed to build task")
            .handler(handler)
            .build()
            .expect("Failed to build task");

        scheduler
            .schedule(task)
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

        let scheduler = create_scheduler::<MockHandler>(Duration::from_millis(100));

        let run_at = Utc::now() + Duration::from_millis(200);
        let task1 = SkedgyTaskBuilder::named("task1")
            .at(run_at)
            .handler(handler1)
            .build()
            .expect("Failed to build task");

        let task2 = SkedgyTaskBuilder::named("task2")
            .r#in(Duration::from_millis(400))
            .handler(handler2)
            .build()
            .expect("Failed to build task");

        scheduler
            .schedule(task1)
            .await
            .expect("Failed to schedule task");

        scheduler
            .schedule(task2)
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
}

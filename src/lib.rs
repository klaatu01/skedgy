use std::{collections::BTreeMap, error::Error, str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use nanoid::nanoid;

pub trait SkedgyHandler: Clone + Send + 'static {
    fn handle(&self) -> impl std::future::Future<Output = ()> + Send;
}

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
            scheduler.run().await;
        });
        Self { tx }
    }

    pub async fn run_at(&mut self, datetime: DateTime<Utc>, handler: T) {
        self.tx
            .send(SkedgyCommand::AddAt(datetime, handler.into()))
            .await
            .unwrap();
    }

    pub async fn run_in(&mut self, duration: Duration, handler: T) {
        let datetime = Utc::now() + duration;
        self.run_at(datetime, handler).await;
    }

    pub async fn cron(&mut self, cron: &str, handler: T) -> Result<(), SkedgyError> {
        if cron::Schedule::from_str(cron).is_err() {
            return Err(SkedgyError::InvalidCron);
        }
        self.tx
            .send(SkedgyCommand::AddCron(cron.to_string(), handler.into()))
            .await
            .unwrap();
        Ok(())
    }
}

#[derive(Debug)]
pub enum SkedgyError {
    SendError,
    RecvError,
    InvalidCron,
}

impl Error for SkedgyError {}

impl std::fmt::Display for SkedgyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkedgyError::SendError => write!(f, "Error sending message to scheduler"),
            SkedgyError::RecvError => write!(f, "Error receiving message from scheduler"),
            SkedgyError::InvalidCron => write!(f, "Invalid cron expression"),
        }
    }
}

pub struct SchedulerLoop<T: SkedgyHandler> {
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

    /// Query the scheduler for all tasks that are scheduled to run before the given instant.
    /// This will remove the tasks from the scheduler.
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
            // Parse the cron expression to create a schedule
            let schedule = match cron::Schedule::from_str(cron) {
                Ok(schedule) => schedule,
                Err(_) => continue, // Skip invalid cron expressions
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

    fn handle_schedules(&mut self, tasks: Vec<SkedgyTask<T>>) {
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

    async fn run(&mut self) {
        let mut last_tick_start = chrono::Utc::now();
        loop {
            log::debug!("Scheduler tick");
            let tick_start = chrono::Utc::now();
            // drain the receiver
            let commands = self.drain_channel().await.unwrap();
            self.handle_commands(commands);

            let tasks = self.query(tick_start);
            self.handle_schedules(tasks);

            let crons = self.query_crons(last_tick_start, tick_start);
            self.handle_schedules(crons);

            let tick_end = chrono::Utc::now();
            let tick_duration = tick_end.signed_duration_since(tick_start).to_std().unwrap();
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

#[derive(Debug, Clone)]
pub struct SkedgyConfig {
    /// The interval at which the scheduler will check for new tasks, this is the minimum interval.
    /// This can also be seen as the scheduler's resolution.
    pub tick_interval: Duration,
}

#[derive(Debug, Clone)]
pub struct SkedgyTask<T: SkedgyHandler> {
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

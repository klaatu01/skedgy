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
    pub fn new(config: SkedgyConfig) -> Result<Self, SkedgyError> {
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
        Ok(Self { tx })
    }

    pub async fn run_at(&mut self, datetime: DateTime<Utc>, handler: T) -> Result<(), SkedgyError> {
        self.tx
            .send(SkedgyCommand::AddAt(datetime, handler.into()))
            .await
            .map_err(|_| SkedgyError::SendError)?;
        Ok(())
    }

    pub async fn run_in(&mut self, duration: Duration, handler: T) -> Result<(), SkedgyError> {
        let datetime = Utc::now() + duration;
        self.run_at(datetime, handler).await
    }

    pub async fn cron(&mut self, cron: &str, handler: T) -> Result<(), SkedgyError> {
        cron::Schedule::from_str(cron).map_err(|_| SkedgyError::InvalidCron)?;
        self.tx
            .send(SkedgyCommand::AddCron(cron.to_string(), handler.into()))
            .await
            .map_err(|_| SkedgyError::SendError)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum SkedgyError {
    SendError,
    RecvError,
    InvalidCron,
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

#[derive(Debug, Clone)]
pub struct SkedgyConfig {
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
        Skedgy::new(config).expect("Failed to create scheduler")
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

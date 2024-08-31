use std::{
    collections::BTreeMap,
    error::Error,
    time::{Duration, Instant},
};

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
            rx,
        };
        tokio::spawn(async move {
            scheduler.run().await;
        });
        Self { tx }
    }

    pub async fn do_at(&mut self, instant: Instant, handler: T) {
        log::debug!(
            "Scheduling task at {}",
            humantime::Duration::from(instant - Instant::now()).to_string()
        );
        self.tx
            .send(SkedgyCommand::Insert(instant, handler))
            .await
            .unwrap();
    }

    pub async fn do_in(&mut self, duration: Duration, handler: T) {
        let instant = Instant::now() + duration;
        self.do_at(instant, handler).await;
    }
}

#[derive(Debug)]
pub enum SkedgyError {
    SendError,
    RecvError,
}

impl Error for SkedgyError {}

impl std::fmt::Display for SkedgyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkedgyError::SendError => write!(f, "Error sending message to scheduler"),
            SkedgyError::RecvError => write!(f, "Error receiving message from scheduler"),
        }
    }
}

pub struct SchedulerLoop<T: SkedgyHandler> {
    config: SkedgyConfig,
    schedules: BTreeMap<Instant, Vec<T>>,
    rx: async_channel::Receiver<SkedgyCommand<T>>,
}

impl<T: SkedgyHandler> SchedulerLoop<T> {
    fn insert(&mut self, instant: Instant, handler: T) {
        self.schedules.entry(instant).or_default().push(handler);
    }

    /// Query the scheduler for all tasks that are scheduled to run before the given instant.
    /// This will remove the tasks from the scheduler.
    fn query(&mut self, end: Instant) -> Vec<T> {
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

    fn handle_schedules(&mut self, tasks: Vec<T>) {
        tokio::spawn(async move {
            for task in tasks {
                task.handle().await;
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
            SkedgyCommand::Insert(instant, handler) => self.insert(instant, handler),
        }
    }

    async fn run(&mut self) {
        loop {
            let tick_start = Instant::now();
            // drain the receiver
            let commands = self.drain_channel().await.unwrap();
            self.handle_commands(commands);

            let tasks = self.query(tick_start);
            self.handle_schedules(tasks);

            let tick_end = Instant::now();
            let tick_duration = tick_end - tick_start;
            if tick_duration < self.config.tick_interval {
                tokio::time::sleep(self.config.tick_interval - tick_duration).await;
            } else {
                log::warn!("Scheduler tick took longer than the tick interval");
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkedgyConfig {
    /// The interval at which the scheduler will check for new tasks, this is the minimum interval.
    /// This can also be seen as the scheduler's resolution.
    pub tick_interval: Duration,
}

enum SkedgyCommand<T> {
    Insert(Instant, T),
}

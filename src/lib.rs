use futures::{channel::mpsc, StreamExt};
use std::{collections::BTreeMap, time::Instant};

pub trait SkedgyHandler {
    async fn handle(&self);
}

pub struct Skedgy<T: SkedgyHandler> {
    tx: mpsc::Sender<T>,
}

pub struct SchedulerLoop<T: SkedgyHandler> {
    config: SkedgyConfig,
    schedules: BTreeMap<Instant, Vec<T>>,
    rx: mpsc::Receiver<SkedgyCommands<T>>,
}

impl SchedulerLoop<T: SkedgyHandler> {
    fn insert(&mut self, instant: Instant, handler: T) {
        self.schedules.entry(instant).or_default().push(handler);
    }

    /// Query the scheduler for all tasks that are scheduled to run before the given instant.
    /// This will remove the tasks from the scheduler.
    fn query(&mut self, end: Instant) -> Vec<T> {
        let mut after = self.schedules.split_off(&end);
        let before = self
            .schedules
            .iter()
            .map(|(_, v)| v)
            .flatten()
            .cloned()
            .collect();
        self.schedules = after;
        before
    }

    async fn run(&mut self) {
        loop {
            let tick_start = Instant::now();
            // drain the receiver
            let commands = self.rx.ready_chunks(100000);
            for command in commands {
                match command {
                    SkedgyCommands::Insert(instant, handler) => {
                        self.insert(instant, handler);
                    }
                }
            }

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

enum SkedgyCommands {
    Insert(Instant, T),
}

impl<T: SkedgyHandler> Skedgy<T> {
    pub fn new(config: SkedgyConfig) -> Self {
        let (tx, rx) = mpsc::unbounded();
    }

    pub async fn at(&mut self, instant: Instant, handler: T) {
        tx.send(SkedgyCommands::Insert(instant, handler)).await;
    }

    pub async fn r#in(&mut self, duration: Duration, handler: T) {
        let instant = Instant::now() + duration;
        tx.send(SkedgyCommands::Insert(instant, handler)).await;
    }
}

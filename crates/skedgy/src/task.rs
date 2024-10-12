use chrono::{DateTime, Utc};
use cron::Schedule;
use std::{sync::Arc, time::Duration};

use crate::DependencyStore;

#[derive(Clone)]
pub enum TaskKind {
    At(DateTime<Utc>),
    In(Duration),
    Cron(Schedule),
}

#[derive(Clone)]
pub struct SkedgyTask {
    pub(crate) id: String,
    pub(crate) kind: TaskKind,
    pub(crate) task: Arc<dyn Task>,
}

impl SkedgyTask {
    pub async fn execute(&self, depenency_store: &DependencyStore) {
        self.task.run(depenency_store).await;
    }
}

pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn futures::Future<Output = T> + Send + 'a>>;

pub trait Task: Send + Sync {
    fn run(&self, depenency_store: &DependencyStore) -> BoxFuture<'_, ()>;
}

mod cron;
mod datetime;
mod duration;
mod handler;
mod named;
mod schedule_builder;

use datetime::DateTime;
use std::borrow::Cow;

use crate::{
    command::SkedgyCommand, config::SkedgyConfig, error::SkedgyError, handler::SkedgyHandler,
    SkedgyContext,
};

use self::cron::Cron;
use self::datetime::IntoDateTime;
use self::duration::{Duration, IntoDuration};
use self::schedule_builder::ScheduleBuilder;
use crate::scheduler::SkedgyScheduler;
use crate::task::SkedgyTask;

/// The main scheduler struct that allows you to schedule tasks to run at specific times, after delays, or using cron expressions.
/// Create a new `Skedgy` instance using the `new` method and schedule tasks using the `run_at`, `run_in`, and `cron` methods.
pub struct Skedgy<Ctx: SkedgyContext> {
    tx: async_channel::Sender<SkedgyCommand<Ctx>>,
    terminate_tx: async_channel::Sender<async_channel::Sender<()>>,
}

impl<Ctx: SkedgyContext> Clone for Skedgy<Ctx> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            terminate_tx: self.terminate_tx.clone(),
        }
    }
}

impl<Ctx: SkedgyContext> Skedgy<Ctx> {
    pub fn new(config: SkedgyConfig, ctx: Ctx) -> Self {
        let (tx, rx) = async_channel::unbounded();
        let (terminate_tx, terminate_rx) = async_channel::bounded(1);
        let mut scheduler = SkedgyScheduler::new(config, rx, terminate_rx, ctx);
        tokio::spawn(async move {
            if let Err(e) = scheduler.run().await {
                log::error!("Scheduler encountered an error: {}", e);
            }
        });
        Self { tx, terminate_tx }
    }

    /// Schedule a task to run at a specific `DateTime<Utc>`.
    /// The `handler` parameter should be a struct that implements the `SkedgyHandler` trait.
    pub(crate) async fn schedule<T: SkedgyHandler<Context = Ctx>>(
        &self,
        task: SkedgyTask<Ctx, T>,
    ) -> Result<(), SkedgyError> {
        self.tx
            .send(SkedgyCommand::Add(task.into()))
            .await
            .map_err(|_| SkedgyError::SendError)?;
        Ok(())
    }

    pub fn named(&self, name: &str) -> named::Named<Ctx> {
        let schedule_builder = ScheduleBuilder::new();
        named::Named {
            skedgy: Cow::Borrowed(self),
            id: name.to_string(),
            schedule_builder,
        }
    }

    /// Schedule a task to run after at a specific `DateTime<Utc>`.
    pub fn datetime(&self, resource: impl IntoDateTime) -> DateTime<Ctx> {
        let schedule_builder = ScheduleBuilder::new();
        DateTime {
            skedgy: Cow::Borrowed(self),
            datetime: resource.into_datetime(),
            schedule_builder,
        }
    }

    /// Schedule a task to run after a specific `Duration`.
    pub fn duration(&self, duration: impl IntoDuration) -> Duration<Ctx> {
        let schedule_builder = ScheduleBuilder::new();
        Duration {
            skedgy: Cow::Borrowed(self),
            duration: duration.into_duration(),
            schedule_builder,
        }
    }

    /// Schedule a task to run using a cron expression.
    pub fn cron(&self, cron: &str) -> Cron<Ctx> {
        let schedule_builder = ScheduleBuilder::new();
        cron::Cron {
            skedgy: Cow::Borrowed(self),
            cron: cron.to_string(),
            schedule_builder,
        }
    }

    /// Remove a cron task by its ID.
    /// The `id` parameter should be the ID returned by the `cron` method.
    pub async fn remove(&self, id: &str) -> Result<(), SkedgyError> {
        self.tx
            .send(SkedgyCommand::<Ctx>::Remove(id.to_string()))
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

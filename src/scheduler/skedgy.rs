use crate::{
    command::SkedgyCommand, config::SkedgyConfig, error::SkedgyError, handler::SkedgyHandler,
};

use super::{SkedgyScheduler, SkedgyState, SkedgyTask};

/// The main scheduler struct that allows you to schedule tasks to run at specific times, after delays, or using cron expressions.
/// Create a new `Skedgy` instance using the `new` method and schedule tasks using the `run_at`, `run_in`, and `cron` methods.
pub struct Skedgy<T: SkedgyHandler> {
    tx: async_channel::Sender<SkedgyCommand<T>>,
    terminate_tx: async_channel::Sender<async_channel::Sender<()>>,
}

impl<T: SkedgyHandler> Skedgy<T> {
    pub fn new(config: SkedgyConfig, ctx: T::Context) -> Self {
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
            .send(SkedgyCommand::<T>::Remove(id.to_string()))
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

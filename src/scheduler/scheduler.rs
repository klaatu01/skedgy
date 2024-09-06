use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use cron::Schedule;

use crate::{
    command::SkedgyCommand,
    config::SkedgyConfig,
    error::SkedgyError,
    handler::SkedgyHandler,
    scheduler::{SkedgyState, TaskKind},
};

use super::task::SkedgyTask;

pub(crate) struct SkedgyScheduler<T: SkedgyHandler> {
    config: SkedgyConfig,
    schedules: BTreeMap<DateTime<Utc>, Vec<SkedgyTask<T>>>,
    crons: Vec<(cron::Schedule, SkedgyTask<T>)>,
    last_cron_run: Option<DateTime<Utc>>,
    rx: async_channel::Receiver<SkedgyCommand<T>>,
    terminate_rx: async_channel::Receiver<async_channel::Sender<()>>,
    ctx: std::sync::Arc<tokio::sync::RwLock<T::Context>>,
}

impl<T: SkedgyHandler> SkedgyScheduler<T> {
    pub(crate) fn new(
        config: SkedgyConfig,
        rx: async_channel::Receiver<SkedgyCommand<T>>,
        terminate_rx: async_channel::Receiver<async_channel::Sender<()>>,
        ctx: T::Context,
    ) -> Self {
        Self {
            config,
            schedules: BTreeMap::new(),
            crons: Vec::new(),
            rx,
            terminate_rx,
            ctx: std::sync::Arc::new(tokio::sync::RwLock::new(ctx)),
            last_cron_run: None,
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

    fn query(&mut self, end: DateTime<Utc>) -> Vec<(DateTime<Utc>, SkedgyTask<T>)> {
        let after = self.schedules.split_off(&end);
        let before = self
            .schedules
            .iter()
            .flat_map(|(datetime, v)| v.iter().map(move |task| (*datetime, task.clone())))
            .collect();
        self.schedules = after;
        before
    }

    fn query_crons(
        &mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<(DateTime<Utc>, SkedgyTask<T>)> {
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
                    Some(
                        runs.into_iter()
                            .map(move |run_time| (run_time, task.clone())),
                    )
                }
            })
            .flatten()
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
        let tasks: Vec<_> = tasks
            .into_iter()
            .map(|task| {
                let ctx = self.ctx.clone();
                (ctx, task)
            })
            .collect();
        tokio::spawn(async move {
            futures::future::join_all(tasks.into_iter().map(|(ctx, task)| {
                let ctx = ctx.clone();
                async move {
                    let ctx = ctx.read().await;
                    task.handler.handle(&ctx).await
                }
            }))
            .await;
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

    fn next_batch(&mut self, from: DateTime<Utc>) -> Vec<(DateTime<Utc>, SkedgyTask<T>)> {
        let look_ahead_time = from + self.config.look_ahead_duration;
        let look_behind_time = from - Duration::milliseconds(1);

        let mut schedule_batch = self.query(look_ahead_time);

        if self.last_cron_run.is_none() || {
            let last_cron_run = self.last_cron_run.unwrap();
            !(look_behind_time <= last_cron_run && last_cron_run <= look_ahead_time)
        } {
            let cron_batch = self.query_crons(look_behind_time, look_ahead_time);

            if !cron_batch.is_empty() {
                self.last_cron_run = Some(cron_batch.first().unwrap().0);
            }
            schedule_batch.extend(cron_batch);
        }

        schedule_batch.sort_by_key(|(datetime, _)| *datetime);
        schedule_batch
    }

    pub fn next_schedule_time(&self) -> Option<DateTime<Utc>> {
        let next_schedule = self
            .schedules
            .first_key_value()
            .map(|(datetime, _)| *datetime);

        let next_cron = self
            .crons
            .iter()
            .filter_map(|(schedule, _)| schedule.upcoming(Utc).next())
            .filter(|time| {
                if let Some(last_cron_run) = self.last_cron_run {
                    time > &last_cron_run
                } else {
                    true
                }
            })
            .min();

        match (next_schedule, next_cron) {
            (Some(schedule), Some(cron)) => Some(schedule.min(cron)),
            (Some(schedule), None) => Some(schedule),
            (None, Some(cron)) => Some(cron),
            (None, None) => None,
        }
    }

    pub(crate) async fn run(&mut self) -> Result<(), SkedgyError> {
        loop {
            log::debug!("Scheduler tick");
            let next_schedule_time = self.next_schedule_time();
            match next_schedule_time {
                None => {
                    log::debug!("No schedules, hybernating.");
                    tokio::select! {
                        _ = self.terminate_rx.recv() => {
                            log::debug!("Terminating scheduler");
                            return Ok(());
                        }
                        next_command = self.rx.recv() => {
                            if let Ok(command) = next_command {
                                self.handle_command(command).await;
                                continue;
                            }
                        }
                    }
                }
                Some(next_schedule_time) => {
                    let sleep_duration: Duration = if next_schedule_time > chrono::Utc::now() {
                        next_schedule_time - chrono::Utc::now()
                    } else {
                        Duration::zero()
                    };

                    log::debug!(
                        "Sleeping for {}",
                        humantime::format_duration(sleep_duration.to_std().unwrap_or_default())
                    );

                    tokio::select! {
                        _ = self.terminate_rx.recv() => {
                            log::debug!("Terminating scheduler");
                            return Ok(());
                        }
                        next_command = self.rx.recv() => {
                            if let Ok(command) = next_command {
                                log::debug!("Hybernation interrupted by command");
                                let mut commands = self.drain_channel().await?;
                                commands.push(command);
                                log::debug!("Received {} commands", commands.len());
                                self.handle_commands(commands).await;
                                continue;
                            }
                        }
                        _ = tokio::time::sleep(sleep_duration.to_std().unwrap_or_default()) => {
                            let next_batch: Vec<(DateTime<Utc>, SkedgyTask<T>)> = self.next_batch(next_schedule_time);
                            let next_tasks = next_batch.iter().map(|(_, task)| task.clone()).collect();
                            self.handle_schedules(next_tasks);
                        }
                    }
                }
            }
        }
    }
}

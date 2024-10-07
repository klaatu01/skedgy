mod dyn_task;

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Timelike, Utc};
use cron::Schedule;

use crate::{
    command::SkedgyCommand, config::SkedgyConfig, error::SkedgyError, handler::Metadata,
    task::TaskKind, SkedgyContext,
};

pub(crate) use self::dyn_task::DynSkedgyTask;

pub(crate) struct SkedgyScheduler<Ctx: SkedgyContext> {
    config: SkedgyConfig,
    schedules: BTreeMap<DateTime<Utc>, Vec<DynSkedgyTask<Ctx>>>,
    crons: Vec<(cron::Schedule, DynSkedgyTask<Ctx>)>,
    last_cron_run: DateTime<Utc>,
    rx: async_channel::Receiver<SkedgyCommand<Ctx>>,
    terminate_rx: async_channel::Receiver<async_channel::Sender<()>>,
    ctx: std::sync::Arc<tokio::sync::RwLock<Ctx>>,
}

impl<Ctx: SkedgyContext> SkedgyScheduler<Ctx> {
    pub(crate) fn new(
        config: SkedgyConfig,
        rx: async_channel::Receiver<SkedgyCommand<Ctx>>,
        terminate_rx: async_channel::Receiver<async_channel::Sender<()>>,
        ctx: Ctx,
    ) -> Self {
        Self {
            config,
            schedules: BTreeMap::new(),
            crons: Vec::new(),
            rx,
            terminate_rx,
            ctx: std::sync::Arc::new(tokio::sync::RwLock::new(ctx)),
            last_cron_run: Utc::now().with_nanosecond(0).unwrap(),
        }
    }

    fn insert(&mut self, datetime: DateTime<Utc>, task: DynSkedgyTask<Ctx>) {
        self.schedules.entry(datetime).or_default().push(task);
    }

    fn insert_cron(&mut self, schedule: Schedule, task: DynSkedgyTask<Ctx>) {
        self.crons.push((schedule, task));
    }

    fn remove_cron(&mut self, id: String) {
        self.crons.retain(|(_, task)| task.id != id);
    }

    fn query(&mut self, end: DateTime<Utc>) -> Vec<(DateTime<Utc>, DynSkedgyTask<Ctx>)> {
        let after = self.schedules.split_off(&end);
        let before = self
            .schedules
            .iter()
            .flat_map(|(datetime, tasks)| {
                tasks
                    .iter()
                    .map(move |task| (*datetime, task.clone()))
                    .collect::<Vec<(DateTime<Utc>, DynSkedgyTask<Ctx>)>>()
            })
            .collect();
        self.schedules = after;
        before
    }

    fn query_crons(
        &mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<(DateTime<Utc>, DynSkedgyTask<Ctx>)> {
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

    async fn drain_channel(&mut self) -> Result<Vec<SkedgyCommand<Ctx>>, SkedgyError> {
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

    fn handle_schedules(&self, tasks: Vec<(DateTime<Utc>, DynSkedgyTask<Ctx>)>) {
        let tasks: Vec<_> = tasks
            .into_iter()
            .map(|(datetime, task)| {
                let ctx = self.ctx.clone();
                let metdata = Metadata {
                    id: task.id.clone(),
                    target_time: datetime,
                };
                (ctx, metdata, task)
            })
            .collect();
        tokio::spawn(async move {
            futures::future::join_all(tasks.into_iter().map(|(ctx, metdata, task)| {
                let ctx = ctx.clone();
                async move {
                    let ctx = ctx.read().await;
                    task.execute(&*ctx, metdata).await;
                }
            }))
            .await;
        });
    }

    async fn handle_commands(&mut self, commands: Vec<SkedgyCommand<Ctx>>) {
        for command in commands {
            self.handle_command(command).await;
        }
    }

    async fn handle_command(&mut self, command: SkedgyCommand<Ctx>) {
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
        }
    }

    fn next_batch(&mut self, from: DateTime<Utc>) -> Vec<(DateTime<Utc>, DynSkedgyTask<Ctx>)> {
        let look_ahead_time = from + self.config.look_ahead_duration;
        let look_behind_time = from - Duration::milliseconds(1);

        let mut schedule_batch = self.query(look_ahead_time);

        let query_for_crons =
            !(self.last_cron_run.gt(&look_behind_time) && self.last_cron_run.lt(&look_ahead_time));

        if query_for_crons {
            let cron_batch = self.query_crons(look_behind_time, look_ahead_time);
            if !cron_batch.is_empty() {
                let next_cron_run = cron_batch.first().unwrap().0;
                schedule_batch.extend(cron_batch);
                self.last_cron_run = next_cron_run;
            }
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
            .filter(|time| time > &self.last_cron_run)
            .min();

        match (next_schedule, next_cron) {
            (Some(schedule), Some(cron)) => Some(schedule.min(cron)),
            (Some(schedule), None) => Some(schedule),
            (None, Some(cron)) => Some(cron),
            (None, None) => None,
        }
    }

    pub(crate) async fn run(&mut self) -> Result<(), SkedgyError> {
        log::info!("{}", self.last_cron_run);
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
                            let next_batch: Vec<(DateTime<Utc>, DynSkedgyTask<Ctx>)> = self.next_batch(next_schedule_time);
                            self.handle_schedules(next_batch);
                        }
                    }
                }
            }
        }
    }
}

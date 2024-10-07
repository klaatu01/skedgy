use chrono::{DateTime, Utc};
use cron::Schedule;
use std::time::Duration;

use crate::{handler::SkedgyHandler, SkedgyContext};

#[derive(Clone)]
pub enum TaskKind {
    At(DateTime<Utc>),
    In(Duration),
    Cron(Schedule),
}

#[derive(Clone)]
pub struct SkedgyTask<Ctx: SkedgyContext, T: SkedgyHandler<Context = Ctx>> {
    pub(crate) id: String,
    pub(crate) kind: TaskKind,
    pub(crate) handler: T,
}

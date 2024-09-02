use std::{collections::BTreeMap, error::Error, str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use cron::Schedule;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

use crate::context::SkedgyContext;

/// A trait for defining task handlers that can be scheduled by the `Skedgy` scheduler.
/// Implement this trait for your task handler and define the task's behavior in the `handle` method.
pub trait SkedgyHandler: Clone + Send + Sync + 'static {
    type Context: SkedgyContext + Send + Sync + 'static;
    fn handle(&self, ctx: Self::Context) -> impl std::future::Future<Output = ()> + Send;
}

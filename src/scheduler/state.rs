use super::task::{SkedgyTask, TaskKind};
use crate::handler::SkedgyHandler;

use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, str::FromStr, time::Duration};

#[derive(Clone, Serialize, Deserialize)]
pub struct SkedgyState<T: SkedgyHandler> {
    tasks: Vec<SkedgyTask<T>>,
}

impl<T: SkedgyHandler> SkedgyState<T> {
    pub fn new(
        crons: Vec<(Schedule, SkedgyTask<T>)>,
        schedules: BTreeMap<DateTime<Utc>, Vec<SkedgyTask<T>>>,
    ) -> Self {
        Self {
            tasks: crons
                .into_iter()
                .map(|(_, task)| task)
                .chain(schedules.values().flat_map(|v| v.iter().cloned()))
                .collect(),
        }
    }

    pub fn crons(&self) -> Vec<(Schedule, SkedgyTask<T>)> {
        self.tasks
            .iter()
            .filter_map(|task| match &task.kind {
                TaskKind::Cron(schedule) => Some((schedule.clone(), task.clone())),
                _ => None,
            })
            .collect()
    }

    pub fn schedules(&self) -> BTreeMap<DateTime<Utc>, Vec<SkedgyTask<T>>> {
        let mut schedules: BTreeMap<DateTime<Utc>, Vec<SkedgyTask<T>>> = BTreeMap::new();
        for task in &self.tasks {
            if let TaskKind::At(datetime) = &task.kind {
                schedules.entry(*datetime).or_default().push(task.clone());
            }
        }
        schedules
    }
}

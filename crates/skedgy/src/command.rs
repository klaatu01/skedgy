use crate::task::SkedgyTask;

pub(crate) enum SkedgyCommand {
    Add(SkedgyTask),
    Remove(String),
}

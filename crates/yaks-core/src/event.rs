use crate::{
    Error,
    task::{Task, TaskID},
};

pub enum Event {
    Prep(Task),
    Started(Task, u64),
    Updated(TaskID, u64),
    Fail(TaskID, Error),
    Finished(TaskID),
}

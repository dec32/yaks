use crate::{
    Error,
    task::{Task, TaskID},
};

pub enum Event {
    Posts(usize),
    Tasks(usize),
    Enqueue(Task),
    Start(TaskID, u64),
    Updated(TaskID, u64),
    Fail(TaskID, Error),
    Finished(TaskID),
}

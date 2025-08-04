use crate::{
    Error,
    task::{Task, TaskID},
};

pub enum Event {
    Posts(usize),
    NoPosts(Error),
    Tasks(usize),
    NoTasks(Error),
    Enqueue(Task),
    Start(TaskID, u64),
    Updated(TaskID, u64),
    Fail(TaskID, Error),
    Finished(TaskID),
}

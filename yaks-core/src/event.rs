use crate::{
    Error,
    task::{Task, TaskID},
};

pub enum Event {
    // fetch profile
    NoProfile(Error),
    Profile,
    // scrape posts
    MorePosts(usize),
    NoPosts(Error),
    NoMorePosts,
    // create tasks
    MoreTasks(usize),
    NoTasks(Error),
    NoMoreTasks,
    // download
    Enqueue(Task),
    Established(TaskID, u64),
    Updated(TaskID, u64),
    Failed(TaskID, Error),
    Finished(TaskID),
    Clear,
}
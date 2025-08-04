use std::collections::HashMap;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::LevelFilter;
use yaks_core::{
    engine::Engine,
    event::Event,
    task::{Task, TaskID},
};

use crate::args::Args;

pub type Result<T = (), E = crate::Error> = std::result::Result<T, E>;
pub type Error = yaks_core::Error;

pub mod args;

const PROGRESS_BAR_TEMPLATE: &str = "{spinner:.green} {msg:<20} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})";
const PROGRESS_BAR_CHARS: &str = "#>-";

const PROGRESS_BAR_ERROR_TEMPLATE: &str = "{spinner:.red} {msg:<20} [{elapsed_precise}] [{wide_bar:.red/red}] {bytes}/{total_bytes} ({eta})";
const PROGRESS_BAR_ERROR_CHARS: &str = "#X-";

#[tokio::main]
async fn main() -> Result {
    // logger
    env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Info)
        .init();

    // args
    let Args {
        platform,
        user_id,
        range,
        cover,
        out,
        template,
        jobs,
    } = Args::from_env()?;

    // tui
    let mut tui = TUI::new();

    // let the app run
    let mut rx = Engine::new()
        .start(platform, user_id, range, cover, out, template, jobs)
        .await?;

    // render from app events
    while let Some(event) = rx.recv().await {
        // render our tui here
        match event {
            Event::Prep(_task) => {}
            Event::Started(task, total) => tui.add_progress(task, total),
            Event::Updated(id, cur) => tui.update_progress(id, cur),
            Event::Fail(id, _error) => tui.freeze_progress(id),
            Event::Finished(id) => tui.del_progress(id),
        }
    }
    Ok(())
}

#[derive(Default)]
pub struct TUI {
    mp: MultiProgress,
    pbs: HashMap<TaskID, ProgressBar>,
}

impl TUI {
    fn new() -> Self {
        let mp = MultiProgress::new();
        mp.set_draw_target(indicatif::ProgressDrawTarget::stderr());
        Self {
            mp,
            pbs: HashMap::new(),
        }
    }

    fn add_progress(&mut self, task: Task, total: u64) {
        let pb = self.mp.add(ProgressBar::new(total));
        let style = ProgressStyle::default_bar()
            .template(PROGRESS_BAR_TEMPLATE)
            .unwrap()
            .progress_chars(PROGRESS_BAR_CHARS);
        pb.set_style(style);
        pb.set_message(format!("{}", task.filename));
        self.pbs.insert(task.id(), pb);
    }

    fn update_progress(&mut self, id: TaskID, cur: u64) {
        let pb = self.pbs.get(&id).unwrap();
        pb.set_position(cur);
    }

    fn freeze_progress(&mut self, id: TaskID) {
        if let Some(pb) = self.pbs.remove(&id) {
            let style = ProgressStyle::default_bar()
                .template(PROGRESS_BAR_ERROR_TEMPLATE)
                .unwrap()
                .progress_chars(PROGRESS_BAR_ERROR_CHARS);
            pb.set_style(style);
            pb.finish();
            self.mp.remove(&pb);
        }
    }

    fn del_progress(&mut self, id: TaskID) {
        if let Some(pb) = self.pbs.remove(&id) {
            pb.finish();
            self.mp.remove(&pb);
        }
    }
}

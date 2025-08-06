use std::{collections::HashMap, time::Duration, u64};

use indicatif::{MultiProgress, ProgressBar};
use log::LevelFilter;
use yaks_core::{engine::Engine, event::Event};

use crate::args::Args;

pub type Result<T = (), E = crate::Error> = std::result::Result<T, E>;
pub type Error = yaks_core::Error;

mod args;
mod style;

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
    let mut bars = HashMap::new();
    let mp = MultiProgress::new();
    mp.set_draw_target(indicatif::ProgressDrawTarget::stderr());

    // let the app run
    let mut rx = Engine::new()
        .start(platform, user_id, range, cover, out, template, jobs)
        .await?;

    // render from app events
    let mut total_tasks = 0;
    let overview = mp.add(ProgressBar::new(0));
    overview.set_message("Fetching profile");
    overview.set_style(style::profile());
    overview.enable_steady_tick(Duration::from_millis(100));
    while let Some(event) = rx.recv().await {
        match event {
            Event::NoProfile(e) => {
                overview.set_style(style::error());
                overview.finish_with_message(format!("Failed to fetch profile :(\n{e}"));
                break;
            }
            Event::Profile => {
                overview.set_style(style::scrape());
                overview.set_message("Scraping posts");
            }
            Event::MorePosts(posts) => {

                overview.inc_length(posts as u64);
            }
            Event::NoPosts(e) => {
                overview.set_style(style::error());
                overview.finish_with_message(format!("Failed to scrape posts :(\n{e}"));
                break;
            }
            Event::NoMorePosts => {
                overview.set_message("Creating tasks");
            }
            Event::MoreTasks(tasks) => {
                total_tasks += tasks;
                overview.inc(1);
            }
            Event::NoTasks(e) => {
                // ignore the failed creations
                overview.set_style(style::error());
                overview.set_message(format!("Failed to create tasks :(\n{e}"));
            }
            Event::NoMoreTasks => {
                // Download really starts here
                overview.disable_steady_tick();
                overview.set_style(style::download());
                overview.set_message("Downloading");
                overview.set_length(total_tasks as u64);
                overview.set_position(0);
            }
            Event::Enqueue(task) => {
                // creating the bar for the tasks
                let bar = mp.add(ProgressBar::new(0));
                bar.set_style(style::enqueued());
                bar.set_length(u64::MAX);
                bar.set_message(format!("{}", task.filename));
                bar.enable_steady_tick(Duration::from_millis(200));
                bars.insert(task.id(), bar);
            }
            Event::Established(id, total) => {
                let bar = bars.get(&id).unwrap();
                bar.set_length(total);
                bar.set_style(style::running());
                bar.disable_steady_tick();
                overview.tick();
            }
            Event::Updated(id, cur) => {
                let bar = bars.get(&id).unwrap();
                bar.set_position(cur);
                overview.tick();
            }
            Event::Failed(id, _error) => {
                if let Some(bar) = bars.remove(&id) {
                    bar.set_style(style::failed());
                    bar.finish();
                }
            }
            Event::Finished(id) => {
                overview.inc(1);
                if let Some(bar) = bars.remove(&id) {
                    bar.finish();
                    mp.remove(&bar);
                }
            }
            Event::Clear => {
                overview.finish_with_message("Clear :)");
                break;
            }
        }
    }
    Ok(())
}

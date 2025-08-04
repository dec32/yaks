use std::{collections::HashMap, time::Duration, u64};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::LevelFilter;
use yaks_core::{engine::Engine, event::Event};

use crate::args::Args;

pub type Result<T = (), E = crate::Error> = std::result::Result<T, E>;
pub type Error = yaks_core::Error;

pub mod args;

const INIT_TEMPLATE: &str = "{spinner:.blue} {msg}";
const DOWNLOAD_TEMPLATE: &str = "[{pos}/{len}] {msg}{spinner:.white}";
const ENQUEUE_TEMPLATE: &str = "{spinner:.dim} {msg:<20} [{elapsed_precise}] [{wide_bar:.dim/dim}]";
const RUNNING_TEMPLATE: &str = "{spinner:.green} {msg:<20} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})";
const FAILED_TEMPLATE: &str =
    "{spinner:.red} {msg:<20} [{elapsed_precise}] [{wide_bar:.red/blue}] {bytes}/{total_bytes}";
const BAR_CHARS: &str = "#>-";

#[tokio::main]
async fn main() -> Result {
    // logger
    env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Info)
        .init();

    // args
    let Args {
        platform,
        uid,
        range,
        cover,
        out,
        template,
        jobs,
    } = Args::from_env()?;

    // tui
    let init_style = ProgressStyle::default_bar()
        .template(INIT_TEMPLATE)
        .unwrap();
    let download_style = ProgressStyle::default_bar()
        .template(DOWNLOAD_TEMPLATE)
        .unwrap()
        .tick_strings(&[".", "..", "...", ""]);
    let ready_style = ProgressStyle::default_bar()
        .template(ENQUEUE_TEMPLATE)
        .unwrap()
        .progress_chars(BAR_CHARS)
        .tick_chars("◜◠◝◞◡◟");
    let running_style = ProgressStyle::default_bar()
        .template(RUNNING_TEMPLATE)
        .unwrap()
        .progress_chars(BAR_CHARS);
    let failed_style = ProgressStyle::default_bar()
        .template(FAILED_TEMPLATE)
        .unwrap()
        .progress_chars(BAR_CHARS)
        .tick_chars("!!");

    let mut bars = HashMap::new();
    let mp = MultiProgress::new();
    mp.set_draw_target(indicatif::ProgressDrawTarget::stderr());

    // let the app run
    let mut rx = Engine::new()
        .start(platform, uid, range, cover, out, template, jobs)
        .await?;

    // render from app events
    let overview = mp.add(ProgressBar::new(0));
    overview.set_message("Collecting posts");
    overview.set_style(init_style);
    overview.enable_steady_tick(Duration::from_millis(50));
    while let Some(event) = rx.recv().await {
        match event {
            Event::Posts(_posts) => {
                overview.set_message(format!("Creating tasks"));
            }
            Event::Tasks(tasks) => {
                overview.set_length(tasks as u64);
                overview.set_message("Downloading");
                overview.set_style(download_style.clone());
                overview.disable_steady_tick();
            }
            Event::Enqueue(task) => {
                let bar = mp.add(ProgressBar::new(0));
                bar.set_style(ready_style.clone());
                bar.set_length(u64::MAX);
                bar.set_message(format!("{}", task.filename));
                bar.enable_steady_tick(Duration::from_millis(200));
                bars.insert(task.id(), bar);
            }
            Event::Start(id, total) => {
                let bar = bars.get(&id).unwrap();
                bar.set_length(total);
                bar.set_style(running_style.clone());
                bar.disable_steady_tick();
                overview.tick();
            }
            Event::Updated(id, cur) => {
                let bar = bars.get(&id).unwrap();
                bar.set_position(cur);
                overview.tick();
            }
            Event::Fail(id, _error) => {
                if let Some(bar) = bars.remove(&id) {
                    bar.set_style(failed_style.clone());
                    bar.finish();
                    mp.remove(&bar);
                }
            }
            Event::Finished(id) => {
                overview.inc(1);
                if let Some(bar) = bars.remove(&id) {
                    bar.finish();
                    mp.remove(&bar);
                }
            }
        }
    }
    overview.set_message("Clear :)");
    Ok(())
}

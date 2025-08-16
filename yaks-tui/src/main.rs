use std::{collections::HashMap, result, time::Duration};

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget};
use yaks_core::{Engine, Event};

use crate::args::Args;

pub type Result<T, E = crate::Error> = result::Result<T, E>;
pub type Error = yaks_core::Error;

mod args;
mod style;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // args
    let Args {
        url,
        range,
        out,
        format,
        workers,
    } = Args::from_conf_then_env().await?;

    // tui
    let mp = MultiProgress::new();
    let mut bars = HashMap::new();
    let mut files = HashMap::new();
    let mut browse_errors = HashMap::new();
    let mut download_errors = HashMap::new();
    let mut waiting = true;

    // let the engine run
    let engine = Engine::default();
    let rx = engine.start(url, range, out, format, workers);

    // create the top banners
    mp.set_draw_target(ProgressDrawTarget::hidden());
    let fetch_profile = mp.add(ProgressBar::new(0));
    fetch_profile.set_style(style::fetch_profile());
    fetch_profile.set_message("Fetching profile...");
    fetch_profile.enable_steady_tick(Duration::from_millis(300));

    let mut scrape_posts = mp.add(ProgressBar::new(0));
    scrape_posts.set_style(style::scrape_posts());
    scrape_posts.set_message("Scraping posts...");
    scrape_posts.enable_steady_tick(Duration::from_millis(100));

    let mut collect_files = mp.add(ProgressBar::new(0));
    collect_files.set_style(style::collect_files());
    collect_files.set_message("Collecting files...");
    collect_files.enable_steady_tick(Duration::from_millis(100));

    let mut download = mp.add(ProgressBar::new(0));
    download.set_style(style::download());
    download.set_message("Waiting...");

    let mut speed = mp.add(ProgressBar::new(0));
    speed.set_style(style::speed());

    mp.remove(&scrape_posts);
    mp.remove(&collect_files);
    mp.remove(&download);
    mp.remove(&speed);
    mp.set_draw_target(ProgressDrawTarget::stderr());

    // render from engine events
    while let Ok(event) = rx.recv().await {
        match event {
            Ok(event) => match event {
                Event::Profile(_profile) => {
                    fetch_profile.set_style(style::finish());
                    fetch_profile.finish_with_message("Profile fetched");
                    scrape_posts = mp.add(scrape_posts);
                }
                Event::Posts(posts) => {
                    scrape_posts.inc(posts as u64);
                    collect_files.inc_length(posts as u64);
                }
                Event::PostsExhausted => {
                    scrape_posts.set_style(style::finish());
                    scrape_posts.finish_with_message("Posts scraped");
                    collect_files = mp.add(collect_files);
                    download = mp.add(download);
                    speed = mp.add(speed);
                }
                Event::Files(new_files) => {
                    collect_files.inc(1);
                    download.inc_length(new_files.len() as u64);
                    for file in new_files {
                        files.insert(file.id(), file);
                    }
                }
                Event::FilesExhausted => {
                    if browse_errors.is_empty() {
                        collect_files.set_style(style::finish());
                        collect_files.finish_with_message("All files collected");
                    } else {
                        collect_files.set_style(style::finish_with_error());
                        collect_files.finish_with_message("Failed to collect all files");
                    }
                }
                Event::Enqueue(id) => {
                    if waiting {
                        download.set_message("Downloading...");
                        waiting = false;
                    }
                    let bar = mp.add(ProgressBar::new(0));
                    bar.set_style(style::enqueued());
                    bar.set_length(u64::MAX);
                    bar.set_message(format!("{}", files.get(&id).unwrap().filename));
                    bar.enable_steady_tick(Duration::from_millis(200));
                    let bar = mp.insert_before(&speed, bar);
                    bars.insert(id, bar);
                }
                Event::Init(id, total) => {
                    let bar = bars.get(&id).unwrap();
                    bar.set_length(total);
                    bar.set_style(style::running());
                    bar.disable_steady_tick();
                }
                Event::Chunk(id, size) => {
                    let bar = bars.get(&id).unwrap();
                    bar.inc(size);
                    speed.inc(size);
                    download.tick();
                }
                Event::Fin(id) => {
                    download.inc(1);
                    mp.remove(&bars.remove(&id).unwrap());
                }
                Event::Clear => {
                    if browse_errors.is_empty() && download_errors.is_empty() {
                        download.set_style(style::finish());
                        download.finish_with_message("Clear :)");
                    } else {
                        download.set_style(style::finish_with_error());
                        download.finish_with_message("Failed to download all files");
                    }
                    break;
                }
            },
            Err(e) => match e {
                Error::Cookies(e) => {
                    fetch_profile.set_style(style::finish_with_error());
                    fetch_profile.finish_with_message(format!("Failed to fetch cookie ({e})"));
                    break;
                }
                Error::Profile(e) => {
                    fetch_profile.set_style(style::finish_with_error());
                    fetch_profile.finish_with_message(format!("Failed to fetch profile ({e})"));
                    break;
                }
                Error::Scrape(e) => {
                    scrape_posts.set_style(style::finish_with_error());
                    scrape_posts.finish_with_message(format!("Failed to scrape posts ({e})"));
                    break;
                }
                Error::Browse(id, e) => {
                    collect_files.set_style(style::error());
                    collect_files.set_message(format!(
                        "Collecting files...(Failed to collect files from post {id} ({e}))"
                    ));
                    browse_errors.insert(id, e);
                }
                Error::Download(id, e) => {
                    let filename = files.get(&id).unwrap().filename.as_ref();
                    download.set_style(style::error());
                    download.set_message(format!(
                        "Downloading...(Failed to download file {filename} ({e}))"
                    ));
                    bars.remove(&id).unwrap();
                    download_errors.insert(id, e);
                }
            },
        }
    }
    Ok(())
}

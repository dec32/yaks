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
        platform,
        user_id,
        range,
        cover,
        out,
        template,
        workers,
    } = Args::from_env()?;

    // tui
    let mut files = HashMap::new();
    let mut bars = HashMap::new();
    let mp = MultiProgress::new();
    mp.set_draw_target(ProgressDrawTarget::hidden());

    // let the engine run
    let engine = Engine::default();
    let rx = engine.start(platform, user_id, range, cover, out, template, workers);

    // create the top 4 banners
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

    // render from app events
    while let Ok(event) = rx.recv().await {
        match event {
            Ok(event) => match event {
                Event::Profile(_profile) => {
                    fetch_profile.set_style(style::clear());
                    fetch_profile.finish_with_message("Profile fetched.");
                    scrape_posts = mp.add(scrape_posts);
                }
                Event::Posts(posts) => {
                    scrape_posts.inc(posts as u64);
                    collect_files.inc_length(posts as u64);
                }
                Event::PostsExhausted => {
                    scrape_posts.set_style(style::clear());
                    scrape_posts.finish_with_message("Posts scraped.");
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
                    mp.remove(&collect_files);
                }
                Event::Enqueue(id) => {
                    download.set_message("Downloading...");
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
                    download.set_style(style::clear());
                    download.finish_with_message("Clear :)");
                    break;
                }
            },
            Err(e) => {
                match e {
                    Error::Profile(e) => {
                        fetch_profile.set_style(style::error());
                        fetch_profile
                            .finish_with_message(format!("Failed to fetch profile :(\n{e}"));
                        break;
                    }
                    Error::Scrape(e) => {
                        scrape_posts.set_style(style::error());
                        scrape_posts.finish_with_message(format!("Failed to scrape posts :(\n{e}"));
                        break;
                    }
                    Error::Browse(id, e) => {
                        collect_files.set_style(style::error());
                        collect_files
                            .set_message(format!("Failed to collect files from post {id} ({e})"));
                    }
                    Error::Download(id, _e) => {
                        // todo: display the message?
                        let bar = bars.remove(&id).unwrap();
                        bar.set_style(style::failed());
                        bar.finish();
                    }
                }
            }
        }
    }
    Ok(())
}

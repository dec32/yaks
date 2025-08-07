use std::{collections::HashMap, result, time::Duration, u64};

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget};
use log::LevelFilter;
use yaks_postcore::{Engine, Event};

use crate::args::Args;

pub type Result<T, E = crate::Error> = result::Result<T, E>;
pub type Error = yaks_postcore::Error;

mod args;
mod style;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
        workers,
    } = Args::from_env()?;

    // tui
    let mut jobs = HashMap::new();
    let mut bars = HashMap::new();
    let mp = MultiProgress::new();
    mp.set_draw_target(ProgressDrawTarget::hidden());

    // let the app run
    let rx = Engine::new()
        .start(platform, user_id, range, cover, out, template, workers);
    
    // create the top 4 banners
    let fetch_profile = mp.add(ProgressBar::new(0));
    fetch_profile.set_style(style::fetch_profile());
    fetch_profile.set_message("Fetching profile");
    fetch_profile.enable_steady_tick(Duration::from_millis(100));

    let mut scrape_posts = mp.add(ProgressBar::new(0));
    scrape_posts.set_style(style::scrape_posts());
    scrape_posts.set_message("Scraping posts");
    scrape_posts.enable_steady_tick(Duration::from_millis(100));

    let mut create_jobs = mp.add(ProgressBar::new(0));
    create_jobs.set_style(style::create_jobs());
    create_jobs.set_message("Collecting files");
    create_jobs.enable_steady_tick(Duration::from_millis(100));

    let mut download = mp.add(ProgressBar::new(0));
    download.set_style(style::download());
    download.set_message("Downloading");

    mp.remove(&scrape_posts);
    mp.remove(&create_jobs);
    mp.remove(&download);
    mp.set_draw_target(ProgressDrawTarget::stderr());
    
    // render from app events
    while let Ok(event) = rx.recv().await {
        match event {
            Ok(event) => {
                match event {
                    Event::Profile(_profile) => {
                        mp.remove(&fetch_profile);
                        scrape_posts = mp.add(scrape_posts);
                    },
                    Event::Posts(posts) => {
                        scrape_posts.inc(posts as u64);
                        create_jobs.inc_length(posts as u64);
                    },
                    Event::PostsExhausted => {
                        mp.remove(&scrape_posts);
                        create_jobs = mp.add(create_jobs);
                        download = mp.add(download);
                    },
                    Event::Jobs(new_jobs) => {
                        create_jobs.inc(1);
                        download.inc_length(new_jobs.len() as u64);
                        for job in new_jobs {
                            jobs.insert(job.id(), job);
                        }
                    },
                    Event::JobExhausted => {
                        mp.remove(&create_jobs);
                    },
                    Event::Enqueue(id) => {
                        let bar = mp.add(ProgressBar::new(0));
                        bar.set_style(style::enqueued());
                        bar.set_length(u64::MAX);
                        bar.set_message(format!("{}", jobs.get(&id).unwrap().filename));
                        bar.enable_steady_tick(Duration::from_millis(200));
                        bars.insert(id, bar);
                    },
                    Event::Init(id, total) => {
                        let bar = bars.get(&id).unwrap();
                        bar.set_length(total);
                        bar.set_style(style::running());
                        bar.disable_steady_tick();
                        download.tick();
                    },
                    Event::Chunk(id, next) => {
                        let bar = bars.get(&id).unwrap();
                        bar.inc(next);
                        download.tick();
                    },
                    Event::Fin(id) => {
                        download.inc(1);
                        mp.remove(&bars.remove(&id).unwrap());
                    },
                    Event::Clear => {
                        download.finish_with_message("Clear :)");
                        break;
                    },
                }
            },
            Err(e) => {
                match e {
                    Error::Profile(e) => {
                        fetch_profile.set_style(style::error());
                        fetch_profile.finish_with_message(format!("Failed to fetch profile :(\n{e}"));
                        break;
                    },
                    Error::Scrape(e) => {
                        scrape_posts.set_style(style::error());
                        scrape_posts.finish_with_message(format!("Failed to scrape posts :(\n{e}"));
                        break;
                    },
                    Error::Browse(id, e) => {
                        create_jobs.set_style(style::error());
                        create_jobs.set_message(format!("Failed to collect files from post {id} :(\n{e}"));
                    },
                    Error::Download(id, _e) => {
                        // todo: display the message?
                        let bar = bars.remove(&id).unwrap();
                        bar.set_style(style::failed());
                        bar.finish();
                    },
                }
            }
        }
    }
    Ok(())
}

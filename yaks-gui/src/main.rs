slint::include_modules!();

use std::{ops::RangeInclusive, path::Path, result};

use async_channel::Receiver;
use yaks_core::{Engine, Event};

pub type Result<T, E = crate::Error> = result::Result<T, E>;
pub type Error = yaks_core::Error;

#[allow(unused)]
#[tokio::main]
async fn main() {
    let ui = MainWindow::new().unwrap();
    let ui_handle = ui.as_weak();
    ui.on_download(move |link, out, format, from, to, workers| {
        let ui = ui_handle.unwrap();
        let engine = Engine::default();
        // check if the link is legal...
        // maybe it's time for the engine to do it?
        let out = Path::new(out.as_str().to_string().leak());
        let format = format.to_string().leak();
        let from = from.parse().unwrap_or(0);
        let to = to.parse().unwrap_or(u64::MAX);
        let range = RangeInclusive::new(from, to);
        let workers = u8::try_from(workers).unwrap();
        // engine.start(platform, user_id, range, out, format, workers);
    });
    ui.run().unwrap();
}

#[allow(unused)]
async fn handle(rx: Receiver<crate::Result<Event>>, ui: MainWindow) {
    while let Ok(event) = rx.recv().await {
        match event {
            Ok(event) => match event {
                Event::Profile => todo!(),
                Event::Posts(_) => todo!(),
                Event::PostsExhausted => todo!(),
                Event::Files(files) => todo!(),
                Event::FilesExhausted => todo!(),
                Event::Enqueue(_) => todo!(),
                Event::Init(_, _) => todo!(),
                Event::Chunk(_, _) => todo!(),
                Event::Fin(_) => todo!(),
                Event::Clear => todo!(),
            },
            Err(e) => match e {
                Error::Profile(e) => todo!(),
                Error::Scrape(e) => todo!(),
                Error::Browse(_, e) => todo!(),
                Error::Download(_, e) => todo!(),
            },
        }
    }
}

use crate::{InjectionEvent, Injector};
use itertools::Itertools;
use regex::Regex;
use std::{fs, path::PathBuf, time::Duration};
use tokio::{sync::watch, time};
use winit::event_loop::EventLoopProxy;

// Playback will enumerate a directory of files with delay, simulating
// some kind of streaming injection.

pub async fn run(
    assets_dir: PathBuf,
    injector: impl Injector,
    exit: watch::Sender<bool>,
    window_proxy: EventLoopProxy<InjectionEvent>,
) {
    let mut interval = time::interval(Duration::from_millis(100));
    let mut exit = exit.subscribe();

    let re = Regex::new(r"(?<instance>.+)\.(?<artifact>.+)\.ply").unwrap();

    loop {
        // Iterate through the assets.  Repeat when list is exhausted.
        for path in fs::read_dir(assets_dir.clone())
            .unwrap()
            .filter_map(|entry| {
                // Filter out non-ply or non-interesting paths.
                let path = entry.unwrap().path();
                if re.is_match(path.to_str().unwrap()) {
                    Some(path)
                } else {
                    None
                }
            })
            .sorted()
        {
            let key = match injector.add(path) {
                Some(key) => key,
                None => continue,
            };

            // Trigger the GUI (main) thread render the new artifact.
            window_proxy
                .send_event(InjectionEvent::Add(key.clone()))
                .ok();

            // Sleep until the next frame.
            // The interval should come from the pose timestamp.
            interval.reset();
            tokio::select! {
                _ = interval.tick() => {}
                Ok(_) = exit.changed() => {
                    // Process is exiting.
                    return
                }
            }
        }
    }
}

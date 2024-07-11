use crate::{InjectionEvent, Injector};
use regex::Regex;
use std::{env, fs, time::Duration};
use tokio::{sync::watch, time};
use winit::event_loop::EventLoopProxy;
use itertools::Itertools;

// Playback will enumerate a directory of files with delay, simulating
// some kind of streaming injection.

pub async fn run(
    injector: impl Injector,
    exit: watch::Sender<bool>,
    window_proxy: EventLoopProxy<InjectionEvent>,
) {
    let mut interval = time::interval(Duration::from_millis(100));
    let mut exit = exit.subscribe();

    let re = Regex::new(r"(?<instance>.+)\.(?<artifact>.+)\.ply").unwrap();
    let assets_dir = env::current_dir().unwrap().join("assets");

    loop {
        let mut current_instance: u32 = 0;

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
            let key = match injector.inject(path) {
                Some(key) => {
                    log::info!("{}", key);
                    key
                },
                None => continue
            };

            // Trigger the GUI (main) thread render the new artifact.
            window_proxy
                .send_event(InjectionEvent::Refresh(key.clone()))
                .ok();

            if current_instance != key.instance {
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

            current_instance = key.instance;
        }
    }
}

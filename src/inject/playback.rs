use crate::{Sequencer, PLY_RE};
use itertools::Itertools;
use regex::Regex;
use std::{fs, path::PathBuf, time::Duration};
use tokio::{sync::watch, time};

// Playback will enumerate a directory of files with delay, simulating
// some kind of streaming injection.

pub async fn run(
    assets_dir: PathBuf,
    sequencer: impl Sequencer,
    delay: Duration,
    filter: Regex,
    exit: watch::Sender<bool>,
) {
    let mut interval = time::interval(delay);
    let mut exit = exit.subscribe();

    let ply_path_re = Regex::new(PLY_RE).unwrap();

    // Iterate through the assets.  Repeat when list is exhausted.
    loop {
        for _key in fs::read_dir(assets_dir.clone())
            .expect(&format!("Cannot read dir {}", assets_dir.display()))
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                // Reject entries that do not match the naming convention.
                ply_path_re.is_match(path.to_str().unwrap())
            })
            .filter(|path| {
                // Reject entries that do not match user supplied filter.
                filter.is_match(path.to_str().unwrap())
            })
            .sorted()
            .filter_map(|path| {
                // The path is good; inject the artifact.
                sequencer.add(&path)
            })
        {
            // For each successful injection, implement the delay.
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

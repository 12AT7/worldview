use crate::window::{DEVICE, QUEUE};
use crate::{model, pipeline::PointCloud, Artifact, Key, PlaybackEvent, RenderArtifact};

use itertools::Itertools;
use ply_rs::{parser::Parser, ply};
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    io::BufRead,
    mem,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::{sync::watch, time::Duration};
use wgpu::util::DeviceExt;
use winit::event_loop::EventLoopProxy;

pub struct Playback {
    pub artifact: HashMap<String, Artifact>,
}

pub type PlaybackLock = Arc<Mutex<Playback>>;

impl Playback {
    pub fn new() -> PlaybackLock {
        let playback = Playback {
            artifact: HashMap::new(),
        };
        Arc::new(Mutex::new(playback))
    }

    pub fn upload(&mut self, key: Key, path: PathBuf) {
        let parse_vertex = Parser::<ply::DefaultElement>::new();

        let f = fs::File::open(path).unwrap();
        let mut f = std::io::BufReader::new(f);
        let header = parse_vertex.read_header(&mut f).unwrap();

        // Remove buffers that are smaller than the new payload.  This
        // will cause reallocation of larger buffers, immediately below.
        if match self.artifact.get(&key.artifact) {
            Some(artifact) => artifact.needs_resize(&header),
            None => false,
        } {
            self.artifact.remove(&key.artifact);
        }

        if !self.artifact.contains_key(&key.artifact) {
            // Allocate new wgpu::Buffers
            let device = match DEVICE.get() {
                Some(device) => device,
                None => {
                    log::debug!("Playback waiting for WGPU initialization");
                    return;
                }
            };

            match Artifact::new(&device, &key, &header) {
                Some(artifact) => {
                    self.artifact.insert(key.artifact.clone(), artifact);
                    log::debug!("Allocated artifact {}", key.artifact)
                }
                None => {
                    log::debug!("Unknown artifact {}", key.artifact);
                }
            };
        }

        let queue = match QUEUE.get() {
            Some(queue) => queue,
            None => {
                log::debug!("Playback waiting for WGPU initialization");
                return;
            }
        };

        match self.artifact.get(&key.artifact) {
            Some(artifact) => {
                artifact.write_buffer(queue, &mut f, &header);
                queue.submit([]);
            }
            None => {}
        }
    }
}

pub async fn run(
    playback: PlaybackLock,
    exit: watch::Sender<bool>,
    window_proxy: EventLoopProxy<PlaybackEvent>,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(10));
    let mut exit = exit.subscribe();

    let re = Regex::new(r"(?<frame>[0-9]+)\.(?<artifact>.+)\.ply").unwrap();
    let assets_dir = env::current_dir().unwrap().join("assets");

    loop {
        let mut current_frame: u32 = 0;

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
            let caps = re.captures(path.to_str().unwrap()).unwrap();

            let key = Key {
                frame: caps["frame"].parse().unwrap(),
                artifact: caps["artifact"].to_string(),
            };

            if key.artifact == "reconstruction.planes" { continue; }

            log::info!("{}", key);

            playback.lock().unwrap().upload(key.clone(), path);

            // Trigger the GUI (main) thread render the new artifact.
            window_proxy
                .send_event(PlaybackEvent::Refresh(key.clone()))
                .ok();

            if current_frame != key.frame {
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

            current_frame = key.frame;
        }
    }
}

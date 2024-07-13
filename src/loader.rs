use crate::{
    window::{DEVICE, QUEUE},
    Artifact, InjectionEvent, Key
};
use ply_rs::{parser::Parser, ply};
use regex::Regex;
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::{mpsc, watch};
use winit::event_loop::EventLoopProxy;

const PLY_RE: &'static str = r"(?<instance>[0-9]+)\.(?<artifact>.+)\.ply";

pub async fn run(
    artifacts: Arc<Mutex<HashMap<Key, Artifact>>>,
    mut rx: mpsc::Receiver<PathBuf>,
    event_loop_proxy: EventLoopProxy<InjectionEvent>,
    exit: watch::Sender<bool>,
) {
    let mut exit = exit.subscribe();

    loop {
        tokio::select! {
            _ = exit.changed() => return,
            path = rx.recv() => {
                load(artifacts.clone(), path.unwrap(), &event_loop_proxy).await;
            }
        }
    }
}

async fn load(
    artifacts: Arc<Mutex<HashMap<Key, Artifact>>>,
    path: PathBuf,
    event_loop_proxy: &EventLoopProxy<InjectionEvent>,
) {
    let ply_re = Regex::new(PLY_RE).expect("invalid regex");
    let parse_header = Parser::<ply::DefaultElement>::new();

    let filename = path.file_name().unwrap().to_str().unwrap();
    let f = File::open(path.clone()).unwrap();
    let mut f = BufReader::new(f);
    let header = match parse_header.read_header(&mut f) {
        Ok(h) => h,
        Err(err) => {
            log::error!("Failed to parse PLY header {}: {:?}", filename, err);
            return;
        }
    };

    let capture = match ply_re.captures(filename) {
        Some(capture) => capture,
        None => {
            log::warn!("cannot match {}", filename);
            return;
        }
    };

    let key = Key {
        instance: None,
        artifact: capture["artifact"].to_string(),
    };

    let mut artifacts = artifacts.lock().unwrap();

    // Remove buffers that are smaller than the new artifact.  This
    // will cause reallocation of larger buffers, immediately below.
    let needs_resize = match artifacts.get(&key) {
        Some(artifact) => artifact.needs_resize(&header),
        None => false,
    };

    if needs_resize {
        artifacts.remove(&key);
    }

    if !artifacts.contains_key(&key) {
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
                artifacts.insert(key.clone(), artifact);
                log::debug!("Allocated artifact {}", key)
            }
            None => {
                log::debug!("Unknown artifact {}", key);
                return;
            }
        };
    }

    let artifact = artifacts.get_mut(&key).unwrap();
    artifact.update_count(&header);
    let queue = QUEUE.get().unwrap(); // Will succeed if DEVICE did.
    artifact.write_buffer(queue, &mut f, &header);
    queue.submit([]);
    event_loop_proxy
        .send_event(InjectionEvent::Add(key.clone()))
        .ok();
}

use crate::{
    InjectionEvent,
    window::{DEVICE, QUEUE},
    Artifact, Injector, Key,
};
use tokio::sync::mpsc;
use winit::event_loop::EventLoopProxy;
use ply_rs::{parser::Parser, ply};
use regex::Regex;
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::PathBuf,
    sync::{Arc, Mutex},
};

// A Sequence is an injector that only keeps the newest artifact, and
// ejects all others.  Consequently, the display will show at most
// one artifact type at a time.

const PLY_RE: &'static str = r"(?<instance>[0-9]+)\.(?<artifact>.+)\.ply";

#[derive(Clone)]
pub struct Sequence {
    pub artifacts: Arc<Mutex<HashMap<Key, Artifact>>>,
    pub ply_re: Regex,
    tx: mpsc::Sender<PathBuf>,
    event_loop_proxy: EventLoopProxy<InjectionEvent>
}

impl Sequence {
    pub fn new(event_loop_proxy: EventLoopProxy<InjectionEvent>) -> Sequence {
        let (tx, _) = mpsc::channel(100);

        Sequence {
            artifacts: Arc::new(Mutex::new(HashMap::new())),
            ply_re: Regex::new(PLY_RE).expect("invalid regex"),
            tx,
            event_loop_proxy
        }
    }

    fn inject(&self, key: Key, path: &PathBuf) {
        let mut artifacts = self.artifacts.lock().unwrap();
        let parse_header = Parser::<ply::DefaultElement>::new();

        let filename = path.file_name().unwrap().to_str().unwrap();
        let f = File::open(path).unwrap();
        let mut f = BufReader::new(f);
        let header = match parse_header.read_header(&mut f) {
            Ok(h) => h,
            Err(err) => {
                log::error!("Failed to parse PLY header {}: {:?}", filename, err);
                return;
            }
        };

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

        let mut artifact = artifacts.get_mut(&key).unwrap();
        artifact.update_count(&header);
        let queue = QUEUE.get().unwrap(); // Will succeed if DEVICE did.
        artifact.write_buffer(queue, &mut f, &header);
        queue.submit([]);

        self.event_loop_proxy.send_event(InjectionEvent::Add(key.clone())).ok();
    }

}

impl Injector for Sequence {
    fn get_artifacts(&self) -> Arc<Mutex<HashMap<Key, Artifact>>> {
        self.artifacts.clone()
    }

    fn add(&self, path: &PathBuf) -> Option<Key> {

        let filename = path.file_name().unwrap().to_str().unwrap();
        let capture = match self.ply_re.captures(filename) {
            Some(capture) => capture,
            None => {
                log::warn!("cannot match {}", filename);
                return None;
            }
        };

        let key = Key {
            instance: None,
            artifact: capture["artifact"].to_string(),
        };

        log::info!("Enqueue {}", key);
        // self.tx.blocking_send(path.clone()).expect("injection worker not running");
        self.inject(key.clone(), path);
        Some(key)
    }

    fn remove(&self, path: &PathBuf) -> Option<Key> {
        let re = Regex::new(r"(?<instance>[0-9]+)\.(?<artifact>.+)\.ply").unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap();
        let capture = match re.captures(filename) {
            Some(capture) => capture,
            None => {
                log::error!("cannot parse {}", filename);
                return None;
            }
        };

        let key = Key {
            instance: None,
            artifact: capture["artifact"].to_string(),
        };

        self.artifacts.lock().unwrap().remove(&key);

        self.event_loop_proxy.send_event(InjectionEvent::Remove(key.clone())).ok();
        Some(key)
    }
}

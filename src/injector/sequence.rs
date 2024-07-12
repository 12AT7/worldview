use crate::{Artifact, Injector, Key, window::{DEVICE, QUEUE}};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    fs::File, io::BufReader
};
use regex::Regex;
use ply_rs::{ply, parser::Parser};

// A Sequence is an injector that only keeps the newest artifact, and
// ejects all others.  Consequently, the display will show at most
// one artifact type at a time.

#[derive(Clone)]
pub struct Sequence {
    pub artifacts: Arc<Mutex<HashMap<Key, Artifact>>>,
}

impl Sequence {
    pub fn new() -> Sequence {
        Sequence {
            artifacts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Injector for Sequence {
    fn get_artifacts(&self) -> Arc<Mutex<HashMap<Key, Artifact>>> {
        self.artifacts.clone()
    }

    fn add(&self, path: PathBuf) -> Option<Key> {
        let mut artifacts = self.artifacts.lock().unwrap();

        let re = Regex::new(r"(?<instance>[0-9]+)\.(?<artifact>.+)\.ply").unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap();
        log::info!("Add {}", filename);
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

        let parse_vertex = Parser::<ply::DefaultElement>::new();

        let f = File::open(path).unwrap();
        let mut f = BufReader::new(f);
        let header = parse_vertex.read_header(&mut f).unwrap();

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
                    return None;
                }
            };

            match Artifact::new(&device, &key, &header) {
                Some(artifact) => {
                    artifacts.insert(key.clone(), artifact);
                    log::debug!("Allocated artifact {}", key)
                }
                None => {
                    log::debug!("Unknown artifact {}", key);
                    return None;
                }
            };
        }

        let artifact = artifacts.get(&key).unwrap();
        let queue = QUEUE.get().unwrap(); // Will succeed if DEVICE did.
        artifact.write_buffer(queue, &mut f, &header);
        queue.submit([]);

        Some(key)
    }

    fn remove(&self, path: PathBuf) -> Option<Key> {
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

        Some(key)
    }

}

use crate::{
    window::{DEVICE, QUEUE},
    Artifact, Element, InjectionEvent, Key, Sequencer, PLY_RE,
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
use winit::event_loop::EventLoopProxy;

// Replace is a sequencer that only keeps the newest artifact, and
// ejects all others.  Consequently, the display will show at most
// one artifact type at a time.

#[derive(Clone)]
pub struct Replace {
    pub artifacts: Arc<Mutex<HashMap<Key, Artifact>>>,
    pub ply_re: Regex,
    event_loop_proxy: EventLoopProxy<InjectionEvent>,
}

impl Replace {
    pub fn new(
        artifacts: Arc<Mutex<HashMap<Key, Artifact>>>,
        event_loop_proxy: EventLoopProxy<InjectionEvent>,
    ) -> Self {
        Self {
            artifacts,
            ply_re: Regex::new(PLY_RE).expect("invalid regex"),
            event_loop_proxy,
        }
    }

    fn inject(&self, key: Key, path: &PathBuf) {
        let parse_header = Parser::<ply::DefaultElement>::new();

        let f = File::open(path).unwrap();
        let mut f = BufReader::new(f);
        let header = match parse_header.read_header(&mut f) {
            Ok(h) => h,
            Err(err) => {
                log::error!("Failed to parse PLY header {}: {:?}", path.display(), err);
                return;
            }
        };

        if header
            .elements
            .get(&Element::Vertex.to_string())
            .unwrap()
            .count
            == 0
        {
            log::warn!("{} is empty; rejecting it", key);
            return;
        }

        // Remove buffers that are smaller than the new artifact.  This
        // will cause reallocation of larger buffers, immediately below.
        let mut artifacts = self.artifacts.lock().unwrap();
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
                    log::debug!("Wait for WGPU initialization");
                    return;
                }
            };

            match Artifact::new(&device, &header) {
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

        let queue = QUEUE.get().unwrap();
        let artifact = artifacts.get_mut(&key).unwrap();
        artifact.update_count(&header);
        artifact.read_ply(&mut f, &header);
        artifact.write_buffer(queue);
        queue.submit([]);

        // New buffers are loaded.  Fire the graphics refresh!
        self.event_loop_proxy
            .send_event(InjectionEvent::Add(key))
            .ok();
    }
}

impl Sequencer for Replace {
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
            instance: capture["instance"].parse::<u32>().ok(),
            artifact: capture["artifact"].to_string(),
        };
        log::debug!("Add {}", key);

        self.inject(key.clone(), path);
        Some(key)
    }

    fn remove(&self, path: &PathBuf) -> Option<Key> {
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
        log::debug!("Remove {}", key);

        self.artifacts.lock().unwrap().remove(&key);

        self.event_loop_proxy
            .send_event(InjectionEvent::Remove(key.clone()))
            .ok();
        Some(key)
    }
}

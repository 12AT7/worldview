use crate::{
    InjectionEvent,
    Artifact, Injector, Key,
};
use winit::event_loop::EventLoopProxy;
use regex::Regex;
use std::{
    collections::HashMap,
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
    // tx: mpsc::Sender<PathBuf>,
    // event_loop_proxy: EventLoopProxy<InjectionEvent>
}

impl Sequence {
    pub fn new(_event_loop_proxy: EventLoopProxy<InjectionEvent>) -> Sequence {
        // let (tx, _) = mpsc::channel(100);

        Sequence {
            artifacts: Arc::new(Mutex::new(HashMap::new())),
            ply_re: Regex::new(PLY_RE).expect("invalid regex"),
            // tx,
            // event_loop_proxy
        }
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
        // self.inject(key.clone(), path);
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

        // self.event_loop_proxy.send_event(InjectionEvent::Remove(key.clone())).ok();
        Some(key)
    }
}

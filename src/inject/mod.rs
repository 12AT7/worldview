use crate::{Artifact, Key};
use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex}};

pub mod playback;
pub mod inotify;
pub mod sequence;

pub use sequence::Sequence;

// Injectors will retrieve artifacts somehow, perhaps from the filesystem
// or a socket, and load wgpu::Buffer(s) with the payload (artifact).  A
// returned Key identifies the buffer so wgpu can find it from the
// rendering thread.

pub trait Injector {
    fn add(&self, path: &PathBuf) -> Option<Key>;
    fn remove(&self, path: &PathBuf) -> Option<Key>;
    fn get_artifacts(&self) -> Arc<Mutex<HashMap<Key, Artifact>>>;
}

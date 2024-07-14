use crate::{Artifact, Key};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub trait Sequencer {
    fn add(&self, path: &PathBuf) -> Option<Key>;
    fn remove(&self, path: &PathBuf) -> Option<Key>;
    fn get_artifacts(&self) -> Arc<Mutex<HashMap<Key, Artifact>>>;
}

pub mod replace;
pub use replace::Replace;

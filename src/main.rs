use clap::{Parser, Subcommand};
use regex::Regex;
use std::{
    collections::HashMap,
    num::ParseIntError,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::watch;
use winit::event_loop::EventLoop;

mod artifact;
mod camera;
mod element;
mod inject;
mod key;
mod model;
mod pipeline;
mod sequence;
mod window;

pub use artifact::{Artifact, ArtifactUniform, RenderArtifact};
pub use camera::{Camera, CameraController, CameraUniform, Projection};
pub use element::{Element, IntoElement};
pub use inject::{inotify, playback};
pub use key::Key;
pub use sequence::Sequencer;
pub use window::WindowState;

// Visualized artifacts (PLY files) must come from somewhere, and we have
// different use cases.  For now, we support dependency injection from
// the filesystem, either as "playback" or using Linux inotify.  Future
// extensions could be gRPC or HTTP/2 servers, or a portable inotify
// replacement (good for Mac and non-Linux platforms).
#[derive(Clone, Subcommand)]
enum DependencyInjector {
    /// Worldview: Enumerate pre-existing directory
    Playback {
        /// Playback directory of PLY files
        path: PathBuf,
        /// Inject a minimum delay between each frame (milliseconds)
        #[clap(value_parser = parse_milliseconds, default_value="100")]
        delay: Duration,
    },
    /// Worldview: Watch live Linux filesystem via inotify (default)
    Notify { path: Option<PathBuf> },
}

#[derive(Parser)]
struct Cli {
    /// Comma separated list of enabled artifact types.  Default: no filter.
    #[clap(short, long, value_delimiter = ',')]
    filter: Option<Vec<String>>,
    #[command(subcommand)]
    injector: Option<DependencyInjector>,
}

#[derive(Debug)]
pub enum InjectionEvent {
    Add(Key),
    Remove(Key),
}

pub type ArtifactsLock = Arc<Mutex<HashMap<Key, Artifact>>>;
const PLY_RE: &'static str = r"(?<instance>[0-9]+)\.(?<artifact>.+)\.ply";

async fn run_dependency_injection<S: Sequencer>(
    cli: &Cli,
    sequencer: S,
    exit: watch::Sender<bool>,
) {
    let cwd = std::env::current_dir().unwrap();

    // Set up a command-line configureable filter, to inject only
    // some artifacts into the renderer.  That can significantly speed up
    // and de-clutter the display, if calculations are dropping too many
    // types of artifacts.
    let filter = Regex::new(&format!(
        "({})",
        cli.filter.clone().unwrap_or(vec![]).join("|")
    ))
    .unwrap();

    match cli.injector.clone() {
        Some(DependencyInjector::Playback { path, delay }) => {
            log::info!(
                "Playback from {}; min refresh {}ms",
                path.display(),
                delay.as_millis()
            );
            playback::run(path, sequencer, delay, filter, exit).await
        }
        Some(DependencyInjector::Notify { path }) => {
            let path = path.clone().unwrap_or(cwd);
            log::info!("Notify from {}", path.display());
            inotify::run(path, sequencer, exit).await
        }
        None => {
            log::info!("Notify from CWD ({})", cwd.display());
            inotify::run(cwd, sequencer, exit).await
        }
    }
}

#[tokio::main(worker_threads = 4)]
async fn main() {
    let cli = Cli::parse();
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("wgpu_hal", log::LevelFilter::Error)
        .filter_module("worldview", log::LevelFilter::Debug)
        .format_timestamp(None)
        .init();

    // Connect to operating system window management (via winit).  The
    // InjectionEvent will be sent to the GUI thread, from the dependency
    // injection thread, to trigger Vulcan refresh.
    let event_loop = EventLoop::<InjectionEvent>::with_user_event()
        .build()
        .unwrap();

    // Provide a signal for all threads to monitor for clean process exit.
    let (exit, _) = watch::channel(false);

    // Artifacts are the producer / consumer queue where the dependency
    // injector (producer) feeds the GUI thread (consumer).
    let artifacts = Arc::new(Mutex::new(HashMap::<Key, Artifact>::new()));

    // The policy when (or if) artifacts get ejected are implemented in
    // the sequencer.  Policies might be "replace" (just show the newest
    // artifact) or "accumulate" (show all the artifacts from all time).
    // It seems to be impossible to use dynamic dispatch into a tokio
    // thread ('static + Send), so use static dispatch for the sequencer
    // here.
    let sequencer = sequence::Replace::new(artifacts.clone(), event_loop.create_proxy());
    let injector_task = tokio::spawn({
        let exit = exit.clone();
        async move { run_dependency_injection(&cli, sequencer, exit).await }
    });

    // Graphics must run on the main thread.  Do not attempt to fight this;
    // the requirement is long baked into some operating systems (i.e.,
    // Linux).  On exit, this future will return cleanly when the window
    // closes via operating system event, or user keypress.
    window::run(artifacts.clone(), event_loop).await;

    log::info!("Exit");

    // Windows are closed, but all other threads need to exit as well.
    exit.send(true).unwrap();
    injector_task.await.unwrap();
}

fn parse_milliseconds(s: &str) -> Result<Duration, ParseIntError> {
    s.parse().map(Duration::from_millis)
}

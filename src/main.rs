use clap::{Parser, Subcommand};
use std::{num::ParseIntError, path::PathBuf, time::Duration};
use tokio::sync::watch;
use winit::event_loop::EventLoop;

mod artifact;
mod camera;
mod element;
mod inject;
mod key;
mod model;
mod pipeline;
mod window;

pub use artifact::{Artifact, ArtifactUniform, RenderArtifact};
pub use camera::{Camera, CameraController, CameraUniform, Projection};
pub use element::{Element, IntoElement};
pub use inject::{inotify, playback, Injector};
pub use key::Key;
pub use window::WindowState;

#[derive(Debug)]
pub enum InjectionEvent {
    Add(Key),
    Remove(Key),
}

#[derive(Subcommand)]
enum Mode {
    Playback {
        path: PathBuf,
        #[clap(value_parser = parse_milliseconds, default_value="100")]
        delay: Duration,
    },
    Notify {
        path: Option<PathBuf>,
    },
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    mode: Option<Mode>,
}

// Feed the visualizer with some kind of dependency injection.  Currently, 
// we have: 
//   1) Playback mode which enumerates a directory (with delay),
//   2) Notify mode which injects frames from Linux inotify() events
async fn run_injector(mode: Option<Mode>, 
    injector: impl Injector,
    exit: watch::Sender<bool>)
{
    let cwd = std::env::current_dir().unwrap();
    match mode {
        Some(Mode::Playback { path, delay }) => {
            log::info!(
                "Playback from {}; min refresh {}ms",
                path.display(),
                delay.as_millis()
            );
            playback::run(path, injector, delay, exit).await
        }
        Some(Mode::Notify { path }) => {
            let path = path.clone().unwrap_or(cwd);
            log::info!("Notify from {}", path.display());
            inotify::run(path, injector, exit).await
        }
        None => {
            log::info!("Notify from CWD ({})", cwd.display());
            inotify::run(cwd, injector, exit).await
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

    let injector = inject::Sequence::new(event_loop.create_proxy());

    // Launch dependency injection thread.
    let injector_task = tokio::spawn({
        let injector = injector.clone();
        let exit = exit.clone();
        async move { run_injector(cli.mode, injector, exit).await }
    });

    // Graphics must run on the main thread.  Do not attempt to fight this!
    // On exit, this future will return cleanly when the window closes
    // via operating system event, or user keypress.
    window::run(injector.clone(), event_loop).await;

    log::info!("Exit");

    // Windows are closed, but all other threads need to exit as well.
    exit.send(true).unwrap();
    injector_task.await.unwrap();
}

fn parse_milliseconds(s: &str) -> Result<Duration, ParseIntError> {
    s.parse().map(Duration::from_millis)
}



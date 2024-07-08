use tokio::sync::watch;
use winit::event_loop::EventLoop;

mod model;
mod key;
mod window;
mod playback;
mod pipeline;
mod artifact;
mod element;
mod camera;

pub use key::Key;
pub use playback::{Playback, PlaybackLock};
pub use artifact::{Artifact, RenderArtifact};
pub use element::Element;
pub use window::WindowState;
pub use camera::{Camera, CameraUniform};

#[derive(Debug)]
pub enum PlaybackEvent {
    Refresh(Key)
}

#[tokio::main(worker_threads = 4)]
async fn main() {
    std::env::set_var("RUST_LOG", "worldview=debug,wgpu_hal=warn,wgpu_core=error");
    env_logger::init();

    let event_loop = EventLoop::<PlaybackEvent>::with_user_event().build().unwrap();
    let playback = Playback::new();

    // Signal all async tasks return for a clean process exit.
    let (exit, _) = watch::channel(false);

    let playback_task = tokio::spawn({
        let playback = playback.clone();
        let exit = exit.clone();
        let window_proxy = event_loop.create_proxy();
        async move {
            let _ = playback::run(playback, exit, window_proxy).await;
        }
    });

    window::run(playback.clone(), event_loop, exit.clone()).await;

    log::info!("Worldview Exit");

    playback_task.await.unwrap();
}

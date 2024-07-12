use tokio::sync::watch;
use winit::event_loop::EventLoop;

mod model;
mod key;
mod window;
mod pipeline;
mod artifact;
mod element;
mod camera;
mod injector;

pub use key::Key;
pub use artifact::{Artifact, RenderArtifact, ArtifactUniform};
pub use element::Element;
pub use window::WindowState;
pub use camera::{Camera, Projection, CameraController, CameraUniform};
pub use injector::{playback, inotify, Injector};

#[derive(Debug)]
pub enum InjectionEvent {
    Add(Key),
    Remove(Key)
}

#[tokio::main(worker_threads = 4)]
async fn main() {
    std::env::set_var("RUST_LOG", "worldview=debug,wgpu_hal=warn,wgpu_core=error");
    env_logger::init();

    let assets_dir = std::env::current_dir().unwrap().join("assets");
    let event_loop = EventLoop::<InjectionEvent>::with_user_event().build().unwrap();
    let injector = injector::Sequence::new();

    // Signal all async tasks return for a clean process exit.
    let (exit, _) = watch::channel(false);

    let injector_task = tokio::spawn({
        let injector = injector.clone();
        let exit = exit.clone();
        let window_proxy = event_loop.create_proxy();
        async move {
            // let _ = playback::run(assets_dir, injector, exit, window_proxy).await;
            let _ = inotify::run(assets_dir, injector, exit, window_proxy).await;
        }
    });

    window::run(injector.clone(), event_loop, exit.clone()).await;

    log::info!("Worldview Exit");

    injector_task.await.unwrap();
}

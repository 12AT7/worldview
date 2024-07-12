use inotify::{EventMask, Inotify, WatchMask};
use std::{fs, path::PathBuf};
use tokio::sync::{watch, mpsc};

// INotify will inject into the visualization, all new files that appear.

pub async fn run(assets_dir: PathBuf, tx: mpsc::Sender<PathBuf>, exit: watch::Sender<bool>) {
    let mut inotify = Inotify::init().unwrap();
    inotify
        .watches()
        .add(
            assets_dir.clone(),
            WatchMask::DELETE | WatchMask::CLOSE_WRITE,
        )
        .unwrap();

    // How the heck to cleanly exit inotify::read_events_blocking()?  It
    // is blocked in the Linux kernel, not tokio, so only a Linux signal
    // can interrupt which feels a bit heavy for this purpose.  We cannot
    // drop or close() it, because we don't own it.  So, drop a sentinal
    // file in the watched directory to signal an exit.
    let mut sentinel_path = assets_dir.clone();
    sentinel_path.push("exit_sentinel");

    // Block on our exit watcher, and write the sentinel when it fires.
    tokio::spawn({
        let mut exit = exit.subscribe();
        let sentinel_path = sentinel_path.clone();
        async move {
            let _ = exit.changed().await;

            // Exit started. Touch a file that the other task will see.
            let _ = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(sentinel_path.clone());

            // Clean up the sentinel.
            fs::remove_file(sentinel_path).unwrap();
        }
    });

    // Read events that were added with `Watches::add` above.
    tokio::task::block_in_place(move || {
            let mut buffer = [0; 1024];
            loop {
                let events = inotify.read_events_blocking(&mut buffer).unwrap();
                for event in events {
                    // Check the exit sentinel for a clean exit.
                    if event.name == Some(sentinel_path.file_name().unwrap()) {
                        return;
                    }

                    let mut path = assets_dir.clone();
                    path.push(event.name.unwrap());

                    match event.mask {
                        EventMask::CLOSE_WRITE => tx.blocking_send(path.clone()).expect("loader not running"), // injector.add(&path),
                        EventMask::DELETE => tx.blocking_send(path.clone()).expect("loader not running"), // injector.remove(&path),
                        _ => {},
                    };
                }
            }
    });
}


use async_process::Command;
use futures::{
    channel::mpsc::{channel, Receiver},
    SinkExt, StreamExt,
};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let path = std::env::args()
        .nth(1)
        .expect("Argument 1 needs to be a path");

    let command = std::env::args()
        .nth(2)
        .expect("Argument 1 needs to be a path");

    log::info!("Watching `{path}`");
    log::info!("Executing `{command}` if any changes are detected in `{path}`");

    futures::executor::block_on(async {
        if let Err(e) = async_watch(path).await {
            println!("error: {:?}", e)
        }
    });
}

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(1);

    let watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                tx.send(res).await.unwrap();
            })
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}

async fn async_watch<P: AsRef<Path>>(path: P) -> notify::Result<()> {
    let (mut watcher, mut rx) = async_watcher()?;

    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    // We skip every first event (seems like a Windows bug).
    let mut skip_counter = 0;

    let path = std::env::args()
        .nth(1)
        .expect("Argument 1 needs to be a path");


    let command = std::env::args()
        .nth(2)
        .expect("Argument 2 needs to be a command to run");

    while let Some(res) = rx.next().await {
        match res {
            Ok(_event) => {
                if skip_counter == 1 {
                    skip_counter -= 1;
                    continue;
                }

                skip_counter += 1;

                let out = Command::new("powershell").current_dir(path.clone()).arg("-command").arg(command.clone()).output().await?;
                if !out.status.success() {
                    let exit_state = String::from_utf8(out.clone().stderr).unwrap();
                    log::error!("{exit_state}");
                } else {
                    let success_state = String::from_utf8(out.stderr).unwrap();
                    log::info!("{success_state}");
                }
            },
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}

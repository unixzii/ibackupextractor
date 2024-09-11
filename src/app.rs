use std::path::Path;

use anyhow::{Context, Result};

use crate::cli::Args;
use crate::db::BackupManifest;
use crate::utils;
use crate::Backup;

mod progress_bar {
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::thread::{Builder as ThreadBuilder, JoinHandle};
    use std::time::Duration;

    use indicatif::{ProgressBar, ProgressStyle};

    use crate::backup::ProgressEvent;

    pub struct ControlPort {
        tx: Sender<Option<ProgressEvent>>,
        join_handle: Option<JoinHandle<()>>,
    }

    impl ControlPort {
        pub fn send(&self, event: ProgressEvent) {
            self.tx.send(Some(event)).unwrap();
        }
    }

    impl Drop for ControlPort {
        fn drop(&mut self) {
            self.tx.send(None).unwrap();
            self.join_handle.take().unwrap().join().unwrap();
        }
    }

    fn thread_main(rx: Receiver<Option<ProgressEvent>>) {
        let spinner_style = ProgressStyle::with_template("{spinner} [{bar:20.white}] {msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .progress_chars("=> ");

        let progress_bar = ProgressBar::new(100);
        progress_bar.set_style(spinner_style);

        loop {
            let Ok(event) = rx.recv_timeout(Duration::from_millis(200)) else {
                // No event at this time, tick the progress bar to keep
                // the animation running.
                progress_bar.tick();
                continue;
            };

            let Some(event) = event else {
                // No more event, exit.
                break;
            };

            update_progress_bar(&progress_bar, event);
        }

        progress_bar.finish_and_clear();
    }

    fn update_progress_bar(progress_bar: &ProgressBar, event: ProgressEvent) {
        match event {
            ProgressEvent::Querying => {
                progress_bar.set_message("Querying database...");
            }
            ProgressEvent::Indexing { indexed, total } => {
                progress_bar
                    .set_message(format!("Creating file system index... ({indexed}/{total})"));
                progress_bar.set_length(total as u64);
                progress_bar.set_position(indexed as u64);
            }
            ProgressEvent::Extracting { extracted, total } => {
                progress_bar.set_message(format!("Extracting files... ({extracted}/{total})"));
                progress_bar.set_length(total as u64);
                progress_bar.set_position(extracted as u64);
            }
            ProgressEvent::Migrating { migrated, total } => {
                progress_bar.set_message(format!("Migrating files... ({migrated}/{total})"));
                progress_bar.set_length(total as u64);
                progress_bar.set_position(migrated as u64);
            }
        }
    }

    pub fn make() -> ControlPort {
        let (tx, rx) = channel();

        let join_handle = ThreadBuilder::new()
            .name("ProgressUIThread".to_owned())
            .spawn(move || thread_main(rx))
            .unwrap();

        ControlPort {
            tx,
            join_handle: Some(join_handle),
        }
    }
}

pub fn run(args: Args) -> Result<()> {
    let backup_dir = args.backup_dir;

    let manifest_path = backup_dir.join("Manifest.db");
    let manifest =
        BackupManifest::open(manifest_path).context("failed to open the manifest database")?;

    let src_backup = Backup::new(backup_dir, manifest, args.copy);

    if args.list_domains {
        let timer = utils::PerfTimer::new();
        let domains = src_backup
            .list_domains()
            .context("failed to list domains")?;
        timer.finish();

        for domain in domains {
            println!("{domain}");
        }
    } else if let Some(migration_dest_dir) = args.migrate_to {
        let timer = utils::PerfTimer::new();
        let pb_port = progress_bar::make();

        let manifest_path = migration_dest_dir.join("Manifest.db");
        let manifest =
            BackupManifest::open(manifest_path).context("failed to open the manifest database")?;

        let dest_backup = Backup::new(migration_dest_dir, manifest, true);
        dest_backup
            .migrate(
                args.domain.as_ref().expect("domain should not be empty"),
                &src_backup,
                |event| {
                    pb_port.send(event);
                },
            )
            .context("failed to migrate files")?;

        // Dispose the progress bar first to prevent it from being
        // clobbered by the timer message.
        drop(pb_port);

        timer.finish();
    } else {
        let timer = utils::PerfTimer::new();
        let pb_port = progress_bar::make();
        src_backup
            .extract_file(
                args.domain.as_ref().expect("domain should not be empty"),
                args.out_dir
                    .as_ref()
                    .map(|p| p as &Path)
                    .expect("out_dir should not be empty"),
                |event| {
                    pb_port.send(event);
                },
            )
            .context("failed to extract files")?;

        drop(pb_port);

        timer.finish();
    }

    Ok(())
}

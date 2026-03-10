use anyhow::{Context, Result};

use crate::Backup;
use crate::cli::{Args, Command};
use crate::db::BackupManifest;
use crate::info::print_backup_info;
use crate::utils::{PerfTimer, format_bytes};
use std::cmp::Reverse;

mod progress_bar {
    use std::sync::mpsc::{Receiver, Sender, channel};
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
    let backup_dir = args.backup_dir();

    let manifest_path = backup_dir.join("Manifest.db");
    let manifest = BackupManifest::open_read_only(manifest_path.clone())
        .context("failed to open the manifest database")?;

    if matches!(&args.command, Command::Info { .. }) {
        print_backup_info(backup_dir, &manifest_path, &manifest)?;
        return Ok(());
    }

    let src_backup = Backup::new(backup_dir, manifest, args.copy_mode());

    match &args.command {
        Command::ListDomains { .. } => {
            let timer = PerfTimer::new();
            let mut domains = src_backup
                .domain_summaries()
                .context("failed to list domains")?;
            domains.sort_by_key(|domain| Reverse(domain.exportable_size));
            timer.finish();

            let formatted_domains: Vec<_> = domains
                .into_iter()
                .map(|domain| {
                    let size_text = format_bytes(domain.exportable_size);
                    (size_text, domain.domain)
                })
                .collect();

            let size_column_width = formatted_domains
                .iter()
                .map(|(size, _)| size.len())
                .max()
                .unwrap_or(0);

            for (size_text, domain_name) in formatted_domains {
                println!("{size_text:>size_column_width$} {domain_name}");
            }
        }
        Command::Migrate {
            dest_backup_dir,
            domain,
            ..
        } => {
            let timer = PerfTimer::new();
            let pb_port = progress_bar::make();

            let manifest_path = dest_backup_dir.join("Manifest.db");
            let manifest = BackupManifest::open(manifest_path)
                .context("failed to open the manifest database")?;

            let dest_backup = Backup::new(dest_backup_dir, manifest, args.copy_mode());
            dest_backup
                .migrate(
                    domain.as_ref().expect("domain should not be empty"),
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
        }
        Command::Extract {
            out_dir,
            domain,
            all,
            ..
        } => {
            let timer = PerfTimer::new();
            let pb_port = progress_bar::make();

            let targets = if *all {
                src_backup
                    .domain_summaries()
                    .context("failed to list domains")?
                    .into_iter()
                    .filter(|summary| summary.exportable_size > 0)
                    .map(|summary| summary.domain)
                    .collect::<Vec<_>>()
            } else {
                vec![domain.as_ref().expect("domain should not be empty").clone()]
            };

            for target in targets {
                src_backup
                    .extract_file(&target, out_dir, |event| {
                        pb_port.send(event);
                    })
                    .with_context(|| format!("failed to extract domain {target}"))?;
            }

            drop(pb_port);

            timer.finish();
        }
        Command::Info { .. } => unreachable!("info command handled before src backup creation"),
    }

    Ok(())
}

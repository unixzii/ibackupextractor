use std::path::{Path, PathBuf};

use clap::{ArgGroup, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

impl Args {
    pub fn backup_dir(&self) -> &Path {
        match &self.command {
            Command::ListDomains { backup_dir } => backup_dir,
            Command::Extract { backup_dir, .. } => backup_dir,
            Command::Migrate { backup_dir, .. } => backup_dir,
            Command::Info { backup_dir } => backup_dir,
        }
    }

    pub fn copy_mode(&self) -> bool {
        match &self.command {
            Command::Extract { link, .. } => !*link,
            Command::Migrate { link, .. } => !*link,
            _ => false,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command()]
    ListDomains {
        /// Path of the backup archive.
        backup_dir: PathBuf,
    },
    Info {
        /// Path of the backup archive whose metadata will be shown.
        backup_dir: PathBuf,
    },
    #[command(
        group(
            ArgGroup::new("extract_target")
                .required(true)
                .multiple(false)
                .args(&["domain", "all"])
        )
    )]
    Extract {
        /// Path of the backup archive.
        backup_dir: PathBuf,

        /// Path of the destination directory for extracted files.
        out_dir: PathBuf,

        /// Domain of the files to extract.
        #[arg(short, long, group = "extract_target")]
        domain: Option<String>,

        /// Extract every domain that contains exportable data.
        #[arg(long, group = "extract_target")]
        all: bool,

        /// Create symbolic links instead of copying files.
        #[arg(short = 'L', long)]
        link: bool,
    },
    Migrate {
        /// Path of the backup archive to migrate from.
        backup_dir: PathBuf,

        /// Path of the backup archive to migrate to.
        dest_backup_dir: PathBuf,

        /// Domain of the files to migrate.
        #[arg(short, long, required = true)]
        domain: Option<String>,

        /// Create symbolic links instead of copying files.
        #[arg(short = 'L', long)]
        link: bool,
    },
}

pub fn parse_args() -> Args {
    Args::parse()
}

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

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
        }
    }

    pub fn copy_mode(&self) -> bool {
        match &self.command {
            Command::Extract { copy, .. } => *copy,
            Command::Migrate { copy, .. } => *copy,
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
    Extract {
        /// Path of the backup archive.
        backup_dir: PathBuf,

        /// Path of the destination directory for extracted files.
        out_dir: PathBuf,

        /// Domain of the files to extract.
        #[arg(short, long, required = true)]
        domain: Option<String>,

        /// Copy the files instead of creating symbolic links.
        #[arg(short, long)]
        copy: bool,
    },
    Migrate {
        /// Path of the backup archive to migrate from.
        backup_dir: PathBuf,

        /// Path of the backup archive to migrate to.
        dest_backup_dir: PathBuf,

        /// Domain of the files to migrate.
        #[arg(short, long, required = true)]
        domain: Option<String>,

        /// Copy the files instead of creating symbolic links.
        #[arg(short, long)]
        copy: bool,
    },
}

pub fn parse_args() -> Args {
    Args::parse()
}

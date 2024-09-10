use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Path of the backup archive.
    pub backup_dir: PathBuf,

    /// Domain of the files to extract.
    #[arg(required = true, conflicts_with = "list_domains")]
    pub domain: Option<String>,

    /// Path of the destination directory for extracted files.
    #[arg(
        short,
        required = true,
        conflicts_with = "list_domains",
        conflicts_with = "migrate_to"
    )]
    pub out_dir: Option<PathBuf>,

    /// List all the domains.
    #[arg(short)]
    pub list_domains: bool,

    /// Copy the files instead of creating symbolic links.
    #[arg(short, conflicts_with = "list_domains", conflicts_with = "migrate_to")]
    pub copy: bool,

    /// Migrate the files to another backup archive.
    #[arg(short, conflicts_with = "list_domains")]
    pub migrate_to: Option<PathBuf>,
}

pub fn parse_args() -> Args {
    Args::parse()
}

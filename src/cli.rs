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
    #[arg(short, required = true, conflicts_with = "list_domains")]
    pub out_dir: Option<PathBuf>,

    /// List all the domains.
    #[arg(short)]
    pub list_domains: bool,

    /// Copy the files instead of creating symbolic links.
    #[arg(short, conflicts_with = "list_domains")]
    pub copy: bool,
}

pub fn parse_args() -> Args {
    Args::parse()
}

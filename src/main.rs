#![feature(assert_matches)]

#[macro_use]
extern crate anyhow;

mod app;
mod backup;
mod cli;
mod db;
mod fs_index;
mod utils;

use backup::Backup;

fn main() {
    let args = cli::parse_args();
    if let Err(err) = app::run(args) {
        let prefix = console::style("error: ").red().bold().to_string();
        println!("{prefix}{err:?}");
    }
}

#![feature(assert_matches)]

#[macro_use]
extern crate anyhow;

mod app;
mod cli;
mod ctx;
mod db;
mod fs_index;
mod utils;

fn main() {
    let args = cli::parse_args();
    if let Err(err) = app::run(args) {
        let prefix = console::style("error: ").red().bold().to_string();
        println!("{prefix}{err:?}");
    }
}

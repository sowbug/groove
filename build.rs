// Copyright (c) 2023 Mike Tsao. All rights reserved.

use clap::{CommandFactory, Parser};

// TODO: this is a pasted copy of some of minidaw.rs. When we ship a real CLI as
// part of the bundled package, refactor its Args code to be shareable.

#[derive(Parser, Debug, Default)]
#[command(author, about, long_about = None)]
struct Args {
    /// Names of files to process. Currently accepts JSON-format projects.
    input: Vec<String>,

    /// Render as WAVE file(s) (file will appear next to source file)
    #[clap(short = 'w', long, value_parser)]
    wav: bool,

    /// Enable debug mode
    #[clap(short = 'd', long, value_parser)]
    debug: bool,

    /// Print version and exit
    #[clap(short = 'v', long, value_parser)]
    version: bool,
}

// https://unix.stackexchange.com/questions/3586/what-do-the-numbers-in-a-man-page-mean
// Picking category 1 as a "user command"
fn main() -> std::io::Result<()> {
    let out_dir = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").ok_or(std::io::ErrorKind::NotFound)?,
    );
    let out_dir = out_dir.join("target");
    let man = clap_mangen::Man::new(Args::command());
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;
    std::fs::write(out_dir.join("groove.1"), buffer)?;
    Ok(())
}

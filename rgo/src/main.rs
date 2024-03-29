use std::fs::OpenOptions;
use std::io::prelude::*;

use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Deserialize, Serialize, Subcommand)]
enum Command {
    List,
    Add{key: String, value: String},
    Remove{key: String},
}

fn main() {
    let args = Args::parse();

    println!("From clap {:?} ", args.command);

    let packed_data = rmp_serde::to_vec(&args.command).unwrap();
    
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open("/tmp/rgo.pipe")
        .expect("Unable to open rgo.pipe");

    file.write_all(&packed_data).unwrap();
}

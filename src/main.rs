use clap::Parser;
use sqframe::{run, Args};

fn main() {
    let args = Args::parse();
    run(args);
}

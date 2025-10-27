mod analysis;
mod parser;

use analysis::compute_liveness;
use clap::Parser;
use l2::*;
use parser::{parse_file, parse_function_file};

#[derive(Parser)]
struct Cli {
    #[arg(short)]
    verbose: bool,

    #[arg(short)]
    generate: u8,

    #[arg(short)]
    liveness: bool,

    source: String,
}

fn main() {
    let cli = Cli::parse();
    let file_name = &cli.source;

    if cli.liveness {
        if let Some(func) = parse_function_file(file_name) {
            let result = compute_liveness(&func);
            print!("{}", result.resolved(&func.interner));
        }
        return;
    }

    if let Some(prog) = parse_file(file_name) {
        for func in &prog.functions {
            let result = compute_liveness(func);
            if cli.liveness {
                println!("{}", result.resolved(&func.interner));
            }
        }
    }
}

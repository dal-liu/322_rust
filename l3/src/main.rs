mod analysis;
mod isel;
mod parser;

use clap::Parser;
use common::DisplayResolved;

use crate::analysis::{compute_liveness, compute_reaching_def};
use crate::isel::{create_contexts, generate_forests};
use crate::parser::parse_file;

#[derive(Parser)]
struct Cli {
    #[arg(short, default_value_t = false)]
    verbose: bool,

    #[arg(short, default_value_t = 1)]
    generate: u8,

    source: String,
}

fn main() {
    let cli = Cli::parse();
    if let Some(prog) = parse_file(&cli.source) {
        if cli.verbose {
            print!("{}", &prog);
        }

        for func in &prog.functions {
            println!("{}", func.name.resolved(&prog.interner));
            // println!("{}", compute_liveness(func).resolved(&prog.interner));
            println!("{}", compute_reaching_def(func).resolved(&prog.interner));
        }
    }
}

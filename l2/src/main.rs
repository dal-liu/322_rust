mod analysis;
mod bitvector;
mod parser;
mod regalloc;

use crate::analysis::compute_liveness;
use crate::parser::{parse_file, parse_function_file, parse_spill_file};
use crate::regalloc::{InterferenceGraph, color_graph, spill_variable_with_display};

use clap::Parser;
use l2::*;

#[derive(Parser)]
struct Cli {
    #[arg(short, default_value_t = false)]
    verbose: bool,

    #[arg(short, default_value_t = 1)]
    generate: u8,

    #[arg(short, default_value_t = false)]
    spill: bool,

    #[arg(short, default_value_t = false)]
    liveness: bool,

    #[arg(short, default_value_t = false)]
    interference: bool,

    source: String,
}

fn main() {
    let cli = Cli::parse();
    let file_name = &cli.source;

    if cli.spill {
        if let Some((mut func, var, prefix)) = parse_spill_file(file_name) {
            let spill_display = spill_variable_with_display(&mut func, &var, &prefix);
            print!("{}", spill_display);
        }
        return;
    }

    if cli.liveness {
        if let Some(func) = parse_function_file(file_name) {
            let liveness = compute_liveness(&func);
            print!("{}", liveness.resolved(&func.interner));
        }
        return;
    }

    if cli.interference {
        if let Some(func) = parse_function_file(file_name) {
            let liveness = compute_liveness(&func);
            let interference = InterferenceGraph::build(&func, &liveness);
            print!("{}", interference.resolved(&func.interner));
        }
        return;
    }

    if let Some(prog) = parse_file(file_name) {
        if cli.verbose {
            print!("{}", &prog);
        }
        for func in &prog.functions {
            let liveness = compute_liveness(func);
            let interference = InterferenceGraph::build(&func, &liveness);
            let coloring = color_graph(interference);
            println!("{}", &coloring.resolved(&func.interner));
        }
    }
}

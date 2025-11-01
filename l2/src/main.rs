mod analysis;
mod bitvector;
mod codegen;
mod parser;
mod regalloc;
mod translation;

use crate::analysis::compute_liveness;
use crate::codegen::generate_code;
use crate::parser::{parse_file, parse_function_file, parse_spill_file};
use crate::regalloc::{allocate_registers, build_interference, spill_with_display};

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
        if let Some((mut prog, var, prefix)) = parse_spill_file(file_name) {
            let spill =
                spill_with_display(&mut prog.functions[0], &var, &prefix, &mut prog.interner);
            print!("{}", spill);
        }
        return;
    }

    if cli.liveness {
        if let Some(prog) = parse_function_file(file_name) {
            let liveness = compute_liveness(&prog.functions[0]);
            print!("{}", liveness.resolved(&prog.interner));
        }
        return;
    }

    if cli.interference {
        if let Some(prog) = parse_function_file(file_name) {
            let liveness = compute_liveness(&prog.functions[0]);
            let interference = build_interference(&prog.functions[0], &liveness);
            print!("{}", interference.resolved(&prog.interner));
        }
        return;
    }

    if let Some(mut prog) = parse_file(file_name) {
        if cli.verbose {
            print!("{}", &prog);
        }
        for func in &mut prog.functions {
            allocate_registers(func, &mut prog.interner);
        }
        generate_code(&prog).unwrap();
    }
}

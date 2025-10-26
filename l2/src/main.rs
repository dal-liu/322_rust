mod analysis;
mod parser;

use analysis::compute_liveness;
use parser::parse_file;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    match parse_file(&args[1]) {
        Some(prog) => {
            for func in &prog.functions {
                let result = compute_liveness(func);
                dbg!(&result);
            }
        }
        None => (),
    }
}

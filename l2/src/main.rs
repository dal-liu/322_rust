mod analysis;
mod parser;

use parser::parse_file;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    match parse_file(&args[1]) {
        Some(mut prog) => {
            for func in &mut prog.functions {
                analysis::compute_targets(func);
            }
            dbg!(&prog);
        }
        None => (),
    }
}

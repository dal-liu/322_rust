mod analysis;
mod parser;

use parser::parse_file;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    match parse_file(&args[1]) {
        Some(mut prog) => {
            analysis::compute_targets(&mut prog);
            dbg!(&prog);
        }
        None => (),
    }
}

mod parser;

use parser::parse_file;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let prog = parse_file(&args[1]).unwrap();
    dbg!(prog);
}

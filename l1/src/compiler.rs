mod codegen;
mod l1;
mod parser;

use codegen::generate_code;
use parser::parse_file;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = parse_file(&args[1]).unwrap();
    generate_code(&program).unwrap();
}

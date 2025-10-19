use std::env;

mod l1;
mod parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    parser::parse_file(&args[1]);
}

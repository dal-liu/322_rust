use std::env;

mod l1;
mod parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = parser::parse_file(&args[1]);
    println!("{}", program.unwrap());
}

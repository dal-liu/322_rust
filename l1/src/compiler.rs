mod codegen;
mod l1;
mod parser;

use clap::Parser;
use codegen::generate_code;
use parser::parse_file;

#[derive(Parser)]
struct Cli {
    source: String,
}

fn main() {
    let cli = Cli::parse();
    let program = parse_file(&cli.source).unwrap();
    generate_code(&program).unwrap();
}

use clap::Parser;

mod codegen;
mod l1;
mod parser;

#[derive(Parser)]
struct Cli {
    source: String,
}

fn main() {
    let cli = Cli::parse();
    let program = parser::parse_file(&cli.source).unwrap();
    codegen::generate_code(&program).unwrap();
}

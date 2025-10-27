mod codegen;
mod parser;

use clap::Parser;
use codegen::generate_code;
use parser::parse_file;

#[derive(Parser)]
struct Cli {
    #[arg(short)]
    verbose: bool,

    #[arg(short)]
    generate: u8,

    source: String,
}

fn main() {
    let cli = Cli::parse();
    if let Some(prog) = parse_file(&cli.source) {
        if cli.verbose {
            print!("{}", &prog);
        }
        if cli.generate == 1 {
            generate_code(&prog).unwrap()
        }
    }
}

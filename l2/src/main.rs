mod parser;

use parser::parse_file;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    match parse_file(&args[1]) {
        Some(prog) => {
            println!("{}", prog);
            ();
        }
        None => (),
    }
}

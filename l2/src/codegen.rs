use crate::translation::translate_program;

use l2::*;
use std::fs::File;
use std::io::{self, BufWriter, Write};

struct CodeGenerator {
    stream: BufWriter<File>,
}

impl CodeGenerator {
    pub fn new() -> io::Result<Self> {
        let file = File::create("prog.L1")?;
        Ok(Self {
            stream: BufWriter::new(file),
        })
    }

    pub fn emit_program(&mut self, prog: &Program) -> io::Result<()> {
        let prog = translate_program(prog);
        write!(self.stream, "{}", prog)
    }

    pub fn finish(mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

pub fn generate_code(prog: &Program) -> io::Result<()> {
    let mut code_generator = CodeGenerator::new()?;
    code_generator.emit_program(&prog)?;
    code_generator.finish()
}

use crate::l1::*;
use std::fmt::format;
use std::fs::File;
use std::io::{self, BufWriter, Write};

pub struct CodeGenerator {
    stream: BufWriter<File>,
}

impl CodeGenerator {
    pub fn new() -> io::Result<Self> {
        let file = File::create("prog.s")?;
        Ok(Self {
            stream: BufWriter::new(file),
        })
    }

    pub fn emit_program(&mut self, prog: &Program) -> io::Result<()> {
        writeln!(
            self.stream,
            "\t.text\n\
         \tglobl go\n\
         go:\n\
         \tpushq %rbx\n\
         \tpushq %rbp\n\
         \tpushq %r12\n\
         \tpushq %r13\n\
         \tpushq %r14\n\
         \tpushq %r15\n\
         \tcall _{}\n\
         \tpopq %r15\n\
         \tpopq %r14\n\
         \tpopq %r13\n\
         \tpopq %r12\n\
         \tpopq %rbp\n\
         \tpopq %rbx\n\
         \tretq",
            &prog.entry_point
        )?;

        for func in &prog.functions {
            self.emit_function(func)?;
        }

        Ok(())
    }

    fn emit_function(&mut self, func: &Function) -> io::Result<()> {
        writeln!(self.stream, "_{}", &func.name)?;

        if func.locals > 0 {
            writeln!(self.stream, "\tsubq ${}", func.locals * 8)?;
        }

        for inst in &func.instructions {
            self.emit_instruction(inst)?;
        }

        Ok(())
    }

    fn emit_instruction(&mut self, inst: &Instruction) -> io::Result<()> {
        match inst {
            Instruction::Assign { dst, src } => {
                if let Value::Register(reg) = src {
                    if dst == reg {
                        return Ok(());
                    }
                }
                writeln!(self.stream, "\tmovq {}, {}", self.format_value(src), dst)
            }
            Instruction::Load { dst, src, offset } => {
                writeln!(self.stream, "\tmovq {}(%{}), %{}", offset, src, dst)
            }
            Instruction::Store { dst, offset, src } => {
                writeln!(
                    self.stream,
                    "\tmovq {}, {}(%{})",
                    self.format_value(src),
                    offset,
                    dst
                )
            }
            Instruction::Shift { lhs, op, rhs } => {
                writeln!(
                    self.stream,
                    "\t{} {}, %{}",
                    match op {
                        ShiftOp::LeftShiftEq => "salq",
                        ShiftOp::RightShiftEq => "sarq",
                    },
                    self.format_value_8(rhs),
                    lhs
                )
            }
            _ => writeln!(self.stream, "TODO"),
        }
    }

    fn format_value(&self, val: &Value) -> String {
        match val {
            Value::Register(r) => format!("%{}", r),
            Value::Number(n) => format!("${}", n),
            Value::Label(s) => format!("${}", s),
            Value::Function(s) => format!("_{}", s),
        }
    }

    fn format_value_8(&self, val: &Value) -> String {
        match val {
            Value::Register(r) => format!("%{}", r.name_8()),
            Value::Number(n) => format!("${}", n),
            Value::Label(s) => format!("${}", s),
            Value::Function(s) => format!("_{}", s),
        }
    }

    pub fn finish(mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

pub fn generate_code(prog: &Program) -> io::Result<()> {
    let mut code_generator = CodeGenerator::new()?;
    code_generator.emit_program(prog)?;
    code_generator.finish()
}

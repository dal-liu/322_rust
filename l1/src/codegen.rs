use l1::*;
use std::fs::File;
use std::io::{self, BufWriter, Write};

struct CodeGenerator {
    stream: BufWriter<File>,
}

impl CodeGenerator {
    pub fn new() -> io::Result<Self> {
        let file = File::create("prog.S")?;
        Ok(Self {
            stream: BufWriter::new(file),
        })
    }

    pub fn emit_program(&mut self, prog: &Program) -> io::Result<()> {
        writeln!(
            self.stream,
            "\t.text\n\
             \t.globl go\n\
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
            prog.entry_point
        )?;

        for func in &prog.functions {
            self.emit_function(func)?;
        }

        Ok(())
    }

    fn emit_function(&mut self, func: &Function) -> io::Result<()> {
        writeln!(self.stream, "_{}:", func.name)?;

        if func.locals > 0 {
            writeln!(self.stream, "\tsubq ${}, %rsp", func.locals * 8)?;
        }

        for inst in &func.instructions {
            self.emit_instruction(func, inst)?;
        }

        Ok(())
    }

    fn emit_instruction(&mut self, func: &Function, inst: &Instruction) -> io::Result<()> {
        use Instruction::*;

        match inst {
            Assign { dst, src } => {
                if let Value::Register(reg) = src {
                    if dst == reg {
                        return Ok(());
                    }
                }
                writeln!(self.stream, "\tmovq {}, %{}", self.format_value(src), dst)
            }
            Load { dst, src, offset } => {
                writeln!(self.stream, "\tmovq {}(%{}), %{}", offset, src, dst)
            }
            Store { dst, offset, src } => {
                writeln!(
                    self.stream,
                    "\tmovq {}, {}(%{})",
                    self.format_value(src),
                    offset,
                    dst
                )
            }
            Arithmetic { dst, op, src } => {
                let arith = match op {
                    ArithmeticOp::PlusEq => "addq",
                    ArithmeticOp::MinusEq => "subq",
                    ArithmeticOp::MultEq => "imulq",
                    ArithmeticOp::AndEq => "andq",
                };
                writeln!(
                    self.stream,
                    "\t{} {}, %{}",
                    arith,
                    self.format_value(src),
                    dst
                )
            }
            Shift { dst, op, src } => {
                let shift = match op {
                    ShiftOp::LeftShiftEq => "salq",
                    ShiftOp::RightShiftEq => "sarq",
                };
                writeln!(
                    self.stream,
                    "\t{} {}, %{}",
                    shift,
                    self.format_value_8_bit(src),
                    dst
                )
            }
            StoreArithmetic {
                dst,
                offset,
                op,
                src,
            } => {
                let arith = match op {
                    ArithmeticOp::PlusEq => "addq",
                    ArithmeticOp::MinusEq => "subq",
                    _ => panic!("store arithmetic invalid op"),
                };
                writeln!(
                    self.stream,
                    "\t{} {}, {}(%{})",
                    arith,
                    self.format_value(src),
                    offset,
                    dst
                )
            }
            LoadArithmetic {
                dst,
                op,
                src,
                offset,
            } => {
                let arith = match op {
                    ArithmeticOp::PlusEq => "addq",
                    ArithmeticOp::MinusEq => "subq",
                    _ => panic!("load arithmetic invalid op"),
                };
                writeln!(self.stream, "\t{} {}(%{}), %{}", arith, offset, src, dst)
            }
            Compare { dst, lhs, op, rhs } => {
                if let (Value::Number(a), Value::Number(b)) = (lhs, rhs) {
                    let res = match op {
                        CompareOp::Less => a < b,
                        CompareOp::LessEq => a <= b,
                        CompareOp::Equal => a == b,
                    };
                    writeln!(self.stream, "\tmovq ${}, %{}", res as u8, dst)
                } else if let Value::Number(n) = lhs {
                    writeln!(self.stream, "\tcmpq ${}, {}", n, self.format_value(rhs))?;
                    let cmp = match op {
                        CompareOp::Less => "setg",
                        CompareOp::LessEq => "setge",
                        CompareOp::Equal => "sete",
                    };
                    let dst_8_bit = self.format_register_8_bit(dst);
                    writeln!(self.stream, "\t{} {}", cmp, dst_8_bit)?;
                    writeln!(self.stream, "\tmovzbq {}, %{}", dst_8_bit, dst)
                } else {
                    writeln!(
                        self.stream,
                        "\tcmpq {}, {}",
                        self.format_value(rhs),
                        self.format_value(lhs)
                    )?;
                    let cmp = match op {
                        CompareOp::Less => "setl",
                        CompareOp::LessEq => "setle",
                        CompareOp::Equal => "sete",
                    };
                    let dst_8_bit = self.format_register_8_bit(dst);
                    writeln!(self.stream, "\t{} {}", cmp, dst_8_bit)?;
                    writeln!(self.stream, "\tmovzbq {}, %{}", dst_8_bit, dst)
                }
            }
            CJump {
                lhs,
                op,
                rhs,
                label,
            } => {
                if let (Value::Number(a), Value::Number(b)) = (lhs, rhs) {
                    let res = match op {
                        CompareOp::Less => a < b,
                        CompareOp::LessEq => a <= b,
                        CompareOp::Equal => a == b,
                    };
                    if res {
                        writeln!(self.stream, "\tjmp _{}", label)
                    } else {
                        Ok(())
                    }
                } else if let Value::Number(n) = lhs {
                    writeln!(self.stream, "\tcmpq ${}, {}", n, self.format_value(rhs))?;
                    let jmp = match op {
                        CompareOp::Less => "jg",
                        CompareOp::LessEq => "jge",
                        CompareOp::Equal => "je",
                    };
                    writeln!(self.stream, "\t{} _{}", jmp, label)
                } else {
                    writeln!(
                        self.stream,
                        "\tcmpq {}, {}",
                        self.format_value(rhs),
                        self.format_value(lhs)
                    )?;
                    let jmp = match op {
                        CompareOp::Less => "jl",
                        CompareOp::LessEq => "jle",
                        CompareOp::Equal => "je",
                    };
                    writeln!(self.stream, "\t{} _{}", jmp, label)
                }
            }
            Label(label) => writeln!(self.stream, "_{}:", label),
            Goto(label) => writeln!(self.stream, "\tjmp _{}", label),
            Return => {
                let stack_size = (func.locals + (func.args - 6).max(0)) * 8;
                if stack_size > 0 {
                    writeln!(self.stream, "\taddq ${}, %rsp", stack_size)?;
                }
                writeln!(self.stream, "\tretq")
            }
            Call { callee, args } => {
                writeln!(self.stream, "\tsubq ${}, %rsp", (args - 6).max(0) * 8 + 8)?;
                let name = match callee {
                    Value::Register(reg) => format!("*%{}", reg),
                    Value::Function(label) => format!("_{}", label),
                    _ => panic!("call invalid callee"),
                };
                writeln!(self.stream, "\tjmp {}", name)
            }
            Print => writeln!(self.stream, "\tcall print"),
            Allocate => writeln!(self.stream, "\tcall allocate"),
            Input => writeln!(self.stream, "\tcall input"),
            TupleError => writeln!(self.stream, "\tcall tuple_error"),
            TensorError(args) => {
                let callee = match args {
                    1 => "array_tensor_error_null",
                    3 => "array_error",
                    4 => "tensor_error",
                    _ => panic!("tensor error invalid args"),
                };
                writeln!(self.stream, "\tcall {}", callee)
            }
            Increment(reg) => writeln!(self.stream, "\tinc %{}", reg),
            Decrement(reg) => writeln!(self.stream, "\tdec %{}", reg),
            LEA {
                dst,
                src,
                offset,
                scale,
            } => {
                writeln!(
                    self.stream,
                    "\tlea (%{}, %{}, {}), %{}",
                    src, offset, scale, dst
                )
            }
        }
    }

    fn format_value(&self, val: &Value) -> String {
        match val {
            Value::Register(r) => format!("%{}", r),
            Value::Number(n) => format!("${}", n),
            Value::Label(s) => format!("$_{}", s),
            Value::Function(s) => format!("$_{}", s),
        }
    }

    fn format_value_8_bit(&self, val: &Value) -> String {
        match val {
            Value::Register(r) => self.format_register_8_bit(r).into(),
            Value::Number(n) => format!("${}", n),
            Value::Label(s) => format!("$_{}", s),
            Value::Function(s) => format!("$_{}", s),
        }
    }

    fn format_register_8_bit(&self, reg: &Register) -> &'static str {
        use Register::*;

        match reg {
            RAX => "%al",
            RBX => "%bl",
            RBP => "%bpl",
            R10 => "%r10b",
            R11 => "%r11b",
            R12 => "%r12b",
            R13 => "%r13b",
            R14 => "%r14b",
            R15 => "%r15b",
            RDI => "%dil",
            RSI => "%sil",
            RDX => "%dl",
            R8 => "%r8b",
            R9 => "%r9b",
            RCX => "%cl",
            RSP => panic!("rsp cannot be 8 bit"),
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

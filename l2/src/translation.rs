use l1;
use l2::*;

pub fn translate_program(prog: &Program) -> l1::Program {
    let l1_functions: Vec<l1::Function> = prog
        .functions
        .iter()
        .map(|func| translate_function(func, &prog.interner))
        .collect();
    l1::Program {
        entry_point: prog.entry_point.clone(),
        functions: l1_functions,
    }
}

fn translate_register(reg: &Value) -> l1::Register {
    use Register::*;
    use l1::Register as L1;

    if let Value::Register(reg) = reg {
        match reg {
            RAX => L1::RAX,
            RDI => L1::RDI,
            RSI => L1::RSI,
            RDX => L1::RDX,
            R8 => L1::R8,
            R9 => L1::R9,
            RCX => L1::RCX,
            RSP => L1::RSP,
            R10 => L1::R10,
            R11 => L1::R11,
            R12 => L1::R12,
            R13 => L1::R13,
            R14 => L1::R14,
            R15 => L1::R15,
            RBP => L1::RBP,
            RBX => L1::RBX,
        }
    } else {
        panic!("not a register");
    }
}

fn translate_value(val: &Value, interner: &Interner<String>) -> l1::Value {
    match val {
        Value::Register(_) => l1::Value::Register(translate_register(val)),
        Value::Number(num) => l1::Value::Number(*num),
        Value::Label(label) => l1::Value::Label(interner.resolve(label.0).to_string()),
        Value::Function(callee) => l1::Value::Function(interner.resolve(callee.0).to_string()),
        Value::Variable(_) => panic!("variables should not exist"),
    }
}

fn translate_instruction(
    inst: &Instruction,
    interner: &Interner<String>,
    locals: i64,
) -> l1::Instruction {
    use Instruction::*;
    use l1::Instruction as L1;

    match inst {
        Assign { dst, src } => L1::Assign {
            dst: translate_register(dst),
            src: translate_value(src, interner),
        },
        Load { dst, src, offset } => L1::Load {
            dst: translate_register(dst),
            src: translate_register(src),
            offset: *offset,
        },
        Store { dst, offset, src } => L1::Store {
            dst: translate_register(dst),
            offset: *offset,
            src: translate_value(src, interner),
        },
        StackArg { dst, offset } => L1::Load {
            dst: translate_register(dst),
            src: l1::Register::RSP,
            offset: locals * 8 + offset,
        },
        Arithmetic { dst, op, src } => {
            let l1_op = match op {
                ArithmeticOp::PlusEq => l1::ArithmeticOp::PlusEq,
                ArithmeticOp::MinusEq => l1::ArithmeticOp::MinusEq,
                ArithmeticOp::MultEq => l1::ArithmeticOp::MultEq,
                ArithmeticOp::AndEq => l1::ArithmeticOp::AndEq,
            };
            L1::Arithmetic {
                dst: translate_register(dst),
                op: l1_op,
                src: translate_value(src, interner),
            }
        }
        Shift { dst, op, src } => {
            let l1_op = match op {
                ShiftOp::LeftShiftEq => l1::ShiftOp::LeftShiftEq,
                ShiftOp::RightShiftEq => l1::ShiftOp::RightShiftEq,
            };
            L1::Shift {
                dst: translate_register(dst),
                op: l1_op,
                src: translate_value(src, interner),
            }
        }
        StoreArithmetic {
            dst,
            offset,
            op,
            src,
        } => {
            let l1_op = match op {
                ArithmeticOp::PlusEq => l1::ArithmeticOp::PlusEq,
                ArithmeticOp::MinusEq => l1::ArithmeticOp::MinusEq,
                _ => panic!("store arithmetic invalid op"),
            };
            L1::StoreArithmetic {
                dst: translate_register(dst),
                offset: *offset,
                op: l1_op,
                src: translate_value(src, interner),
            }
        }
        LoadArithmetic {
            dst,
            op,
            src,
            offset,
        } => {
            let l1_op = match op {
                ArithmeticOp::PlusEq => l1::ArithmeticOp::PlusEq,
                ArithmeticOp::MinusEq => l1::ArithmeticOp::MinusEq,
                _ => panic!("store arithmetic invalid op"),
            };
            L1::LoadArithmetic {
                dst: translate_register(dst),
                op: l1_op,
                src: translate_register(src),
                offset: *offset,
            }
        }
        Compare { dst, lhs, op, rhs } => {
            let l1_op = match op {
                CompareOp::Less => l1::CompareOp::Less,
                CompareOp::LessEq => l1::CompareOp::LessEq,
                CompareOp::Equal => l1::CompareOp::Equal,
            };
            L1::Compare {
                dst: translate_register(dst),
                lhs: translate_value(lhs, interner),
                op: l1_op,
                rhs: translate_value(rhs, interner),
            }
        }
        CJump {
            lhs,
            op,
            rhs,
            label,
        } => {
            let l1_op = match op {
                CompareOp::Less => l1::CompareOp::Less,
                CompareOp::LessEq => l1::CompareOp::LessEq,
                CompareOp::Equal => l1::CompareOp::Equal,
            };
            L1::CJump {
                lhs: translate_value(lhs, interner),
                op: l1_op,
                rhs: translate_value(rhs, interner),
                label: interner.resolve(label.0).to_string(),
            }
        }
        Label(label) => L1::Label(interner.resolve(label.0).to_string()),
        Goto(label) => L1::Goto(interner.resolve(label.0).to_string()),
        Return => L1::Return,
        Call { callee, args } => L1::Call {
            callee: translate_value(callee, interner),
            args: *args,
        },
        Print => L1::Print,
        Input => L1::Input,
        Allocate => L1::Allocate,
        TupleError => L1::TupleError,
        TensorError(args) => L1::TensorError(*args),
        Increment(reg) => L1::Increment(translate_register(reg)),
        Decrement(reg) => L1::Decrement(translate_register(reg)),
        LEA {
            dst,
            src,
            offset,
            scale,
        } => L1::LEA {
            dst: translate_register(dst),
            src: translate_register(src),
            offset: translate_register(offset),
            scale: *scale,
        },
    }
}

fn translate_function(func: &Function, interner: &Interner<String>) -> l1::Function {
    let l1_instructions: Vec<l1::Instruction> = func
        .basic_blocks
        .iter()
        .flat_map(|block| {
            block
                .instructions
                .iter()
                .map(|inst| translate_instruction(inst, interner, func.locals))
        })
        .collect();
    l1::Function {
        name: interner.resolve(func.name.0).to_string(),
        args: func.args,
        locals: func.locals,
        instructions: l1_instructions,
    }
}

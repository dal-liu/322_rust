use chumsky::prelude::*;
use l2::*;
use std::fs;

fn separators<'src>() -> impl Parser<'src, &'src str, ()> + Copy {
    one_of(" \t").repeated()
}

fn comment<'src>() -> impl Parser<'src, &'src str, ()> {
    just("//").ignore_then(none_of('\n').repeated()).padded()
}

fn register<'src>() -> impl Parser<'src, &'src str, Register> {
    choice((
        just("rax").to(Register::RAX),
        just("rdi").to(Register::RDI),
        just("rsi").to(Register::RSI),
        just("rdx").to(Register::RDX),
        just("r8").to(Register::R8),
        just("r9").to(Register::R9),
        just("rcx").to(Register::RCX),
        just("rsp").to(Register::RSP),
    ))
    .padded_by(separators())
}

fn number<'src>() -> impl Parser<'src, &'src str, i64> {
    just('+')
        .to(1)
        .or(just('-').to(-1))
        .or_not()
        .map(|opt| opt.unwrap_or(1))
        .then(text::int(10).from_str::<i128>().unwrapped())
        .map(|(sign, magnitude)| (sign * magnitude) as i64)
        .padded_by(separators())
}

fn function_name<'src>() -> impl Parser<'src, &'src str, String> {
    just('@')
        .ignore_then(text::ascii::ident().map(|s: &str| s.to_string()))
        .padded_by(separators())
}

fn label_name<'src>() -> impl Parser<'src, &'src str, String> {
    just(':')
        .ignore_then(text::ascii::ident().map(|s: &str| s.to_string()))
        .padded_by(separators())
}

fn variable<'src>() -> impl Parser<'src, &'src str, String> {
    just('%')
        .ignore_then(text::ascii::ident().map(|s: &str| s.to_string()))
        .padded_by(separators())
}

fn value<'src>() -> impl Parser<'src, &'src str, Value> {
    choice((
        register().map(|reg| Value::Register(reg)),
        number().map(|num| Value::Number(num)),
        function_name().map(|callee| Value::Function(callee)),
        label_name().map(|label| Value::Label(label)),
        variable().map(|var| Value::Variable(var)),
    ))
    .padded_by(separators())
}

fn arithmetic_op<'src>() -> impl Parser<'src, &'src str, ArithmeticOp> {
    choice((
        just("+=").to(ArithmeticOp::PlusEq),
        just("-=").to(ArithmeticOp::MinusEq),
        just("*=").to(ArithmeticOp::MultEq),
        just("&=").to(ArithmeticOp::AndEq),
    ))
    .padded_by(separators())
}

fn shift_op<'src>() -> impl Parser<'src, &'src str, ShiftOp> {
    choice((
        just("<<=").to(ShiftOp::LeftShiftEq),
        just(">>=").to(ShiftOp::RightShiftEq),
    ))
    .padded_by(separators())
}

fn compare_op<'src>() -> impl Parser<'src, &'src str, CompareOp> {
    choice((
        just("<=").to(CompareOp::LessEq),
        just("<").to(CompareOp::Less),
        just("=").to(CompareOp::Equal),
    ))
    .padded_by(separators())
}

fn instruction<'src>() -> impl Parser<'src, &'src str, Instruction> {
    let arrow = just("<-").padded_by(separators());

    let mem = just("mem").padded_by(separators());

    let call_keyword = just("call").padded_by(separators());

    let stack_arg_keyword = just("stack-arg").padded_by(separators());

    let assign = value()
        .then_ignore(arrow)
        .then(value())
        .map(|(dst, src)| Instruction::Assign { dst, src });

    let load = value()
        .then_ignore(arrow.then_ignore(mem))
        .then(value())
        .then(number())
        .map(|((dst, src), offset)| Instruction::Load { dst, src, offset });

    let store = mem
        .ignore_then(value())
        .then(number())
        .then_ignore(arrow)
        .then(value())
        .map(|((dst, offset), src)| Instruction::Store { dst, offset, src });

    let stack_arg = value()
        .then_ignore(arrow.then(stack_arg_keyword))
        .then(number())
        .map(|(dst, offset)| Instruction::StackArg { dst, offset });

    let arithmetic = value()
        .then(arithmetic_op())
        .then(value())
        .map(|((lhs, op), rhs)| Instruction::Arithmetic { lhs, op, rhs });

    let shift = value()
        .then(shift_op())
        .then(value())
        .map(|((lhs, op), rhs)| Instruction::Shift { lhs, op, rhs });

    let store_arithmetic = mem
        .ignore_then(value())
        .then(number())
        .then(arithmetic_op())
        .then(value())
        .map(|(((dst, offset), op), src)| Instruction::StoreArithmetic {
            dst,
            offset,
            op,
            src,
        });

    let load_arithmetic = value()
        .then(arithmetic_op())
        .then_ignore(mem)
        .then(value())
        .then(number())
        .map(|(((dst, op), src), offset)| Instruction::LoadArithmetic {
            dst,
            op,
            src,
            offset,
        });

    let compare = value()
        .then_ignore(arrow)
        .then(value())
        .then(compare_op())
        .then(value())
        .map(|(((dst, lhs), op), rhs)| Instruction::Compare { dst, lhs, op, rhs });

    let cjump = just("cjump")
        .padded_by(separators())
        .ignore_then(value())
        .then(compare_op())
        .then(value())
        .then(label_name())
        .map(|(((lhs, op), rhs), label)| Instruction::CJump {
            lhs,
            op,
            rhs,
            label,
        });

    let label_inst = label_name().map(|label| Instruction::Label(label));

    let goto = just("goto")
        .padded_by(separators())
        .ignore_then(label_name())
        .map(|label| Instruction::Goto(label));

    let return_inst = just("return")
        .padded_by(separators())
        .to(Instruction::Return);

    let call_inst = call_keyword
        .ignore_then(value())
        .then(number())
        .map(|(callee, args)| Instruction::Call { callee, args });

    let print = call_keyword
        .then(just("print").padded_by(separators()))
        .then(just('1').padded_by(separators()))
        .to(Instruction::Print);

    let input = call_keyword
        .then(just("input").padded_by(separators()))
        .then(just('0').padded_by(separators()))
        .to(Instruction::Input);

    let allocate = call_keyword
        .then(just("allocate").padded_by(separators()))
        .then(just('2').padded_by(separators()))
        .to(Instruction::Allocate);

    let tuple_error = call_keyword
        .then(just("tuple-error").padded_by(separators()))
        .then(just('3').padded_by(separators()))
        .to(Instruction::TupleError);

    let tensor_error = call_keyword
        .ignore_then(just("tensor-error").padded_by(separators()))
        .ignore_then(
            text::int(10)
                .from_str::<u8>()
                .unwrapped()
                .filter(|&n| n == 1 || n == 3 || n == 4),
        )
        .map(|args| Instruction::TensorError(args));

    let increment = value()
        .then_ignore(just("++").padded_by(separators()))
        .map(|reg| Instruction::Increment(reg));

    let decrement = value()
        .then_ignore(just("--").padded_by(separators()))
        .map(|reg| Instruction::Decrement(reg));

    let lea = value()
        .then_ignore(just('@').padded_by(separators()))
        .then(value())
        .then(value())
        .then(
            text::int(10)
                .from_str::<u8>()
                .unwrapped()
                .filter(|&n| n == 1 || n == 2 || n == 4 || n == 8),
        )
        .map(|(((dst, src), offset), scale)| Instruction::LEA {
            dst,
            src,
            offset,
            scale,
        });

    choice((
        compare,
        assign,
        load,
        store,
        stack_arg,
        arithmetic,
        shift,
        store_arithmetic,
        load_arithmetic,
        cjump,
        label_inst,
        goto,
        return_inst,
        call_inst,
        print,
        input,
        allocate,
        tuple_error,
        tensor_error,
        increment,
        decrement,
        lea,
    ))
    .padded_by(comment().repeated())
    .padded()
}

fn function<'src>() -> impl Parser<'src, &'src str, Function> {
    just('(')
        .padded_by(comment().repeated())
        .padded()
        .ignore_then(function_name().padded_by(comment().repeated()).padded())
        .then(number().padded_by(comment().repeated()).padded())
        .then(
            instruction()
                .repeated()
                .at_least(1)
                .collect::<Vec<Instruction>>(),
        )
        .then_ignore(just(')').padded_by(comment().repeated()).padded())
        .map(|((name, args), instructions)| Function {
            name,
            args,
            basic_blocks: collect_basic_blocks(instructions),
        })
}

fn program<'src>() -> impl Parser<'src, &'src str, Program> {
    just('(')
        .padded_by(comment().repeated())
        .padded()
        .ignore_then(function_name().padded_by(comment().repeated()).padded())
        .then(function().repeated().at_least(1).collect::<Vec<Function>>())
        .then_ignore(just(')').padded_by(comment().repeated()).padded())
        .map(|(entry_point, functions)| Program {
            entry_point,
            functions,
        })
        .then_ignore(any().repeated())
}

fn collect_basic_blocks(instructions: Vec<Instruction>) -> Vec<BasicBlock> {
    let mut blocks = vec![BasicBlock {
        instructions: vec![],
    }];

    for inst in instructions {
        let block = blocks.last_mut().unwrap();

        match inst {
            Instruction::CJump { .. } | Instruction::Goto(_) | Instruction::Return => {
                block.instructions.push(inst);
                blocks.push(BasicBlock {
                    instructions: vec![],
                });
            }
            Instruction::Label(_) => {
                if block.instructions.is_empty() {
                    block.instructions.push(inst);
                } else {
                    blocks.push(BasicBlock {
                        instructions: vec![inst],
                    });
                }
            }
            _ => {
                block.instructions.push(inst);
            }
        }
    }

    if blocks
        .last()
        .map_or(false, |block| block.instructions.is_empty())
    {
        blocks.pop();
    }

    blocks
}

pub fn parse_file<'a>(file_name: &'a str) -> Option<Program> {
    let file_input = fs::read_to_string(file_name).unwrap();
    program().parse(&file_input).into_output()
}

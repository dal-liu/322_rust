use crate::l1::*;
use chumsky::prelude::*;
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
        just("rbx").to(Register::RBX),
        just("rbp").to(Register::RBP),
        just("r10").to(Register::R10),
        just("r11").to(Register::R11),
        just("r12").to(Register::R12),
        just("r13").to(Register::R13),
        just("r14").to(Register::R14),
        just("r15").to(Register::R15),
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
        .then(text::int(10).from_str::<i64>().unwrapped())
        .map(|(sign, magnitude)| sign * magnitude)
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

fn value<'src>() -> impl Parser<'src, &'src str, Value> {
    choice((
        register().map(|reg| Value::Register(reg)),
        number().map(|num| Value::Number(num)),
        function_name().map(|callee| Value::Function(callee)),
        label_name().map(|label| Value::Label(label)),
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

    let assign = register()
        .then_ignore(arrow)
        .then(value())
        .map(|(dst, src)| Instruction::Assign { dst, src });

    let load = register()
        .then_ignore(arrow.then_ignore(mem))
        .then(register())
        .then(text::int(10).from_str::<i64>().unwrapped())
        .map(|((dst, src), offset)| Instruction::Load { dst, src, offset });

    let store = mem
        .ignore_then(register())
        .then(number())
        .then_ignore(arrow)
        .then(value())
        .map(|((dst, offset), src)| Instruction::Store { dst, offset, src });

    let arithmetic = register()
        .then(arithmetic_op())
        .then(value())
        .map(|((lhs, op), rhs)| Instruction::Arithmetic { lhs, op, rhs });

    let shift = register()
        .then(shift_op())
        .then(value())
        .map(|((lhs, op), rhs)| Instruction::Shift { lhs, op, rhs });

    let store_arithmetic = mem
        .ignore_then(register())
        .then(text::int(10).from_str::<i64>().unwrapped())
        .then(arithmetic_op())
        .then(value())
        .map(|(((dst, offset), op), src)| Instruction::StoreArithmetic {
            dst,
            offset,
            op,
            src,
        });

    let load_arithmetic = register()
        .then(arithmetic_op())
        .then_ignore(mem)
        .then(register())
        .then(text::int(10).from_str::<i64>().unwrapped())
        .map(|(((dst, op), src), offset)| Instruction::LoadArithmetic {
            dst,
            op,
            src,
            offset,
        });

    let compare = register()
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

    let increment = register()
        .then_ignore(just("++").padded_by(separators()))
        .map(|reg| Instruction::Increment(reg));

    let decrement = register()
        .then_ignore(just("--").padded_by(separators()))
        .map(|reg| Instruction::Decrement(reg));

    let lea = register()
        .then_ignore(just('@').padded_by(separators()))
        .then(register())
        .then(register())
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
        .then(number().padded_by(comment().repeated()).padded())
        .then(
            instruction()
                .repeated()
                .at_least(1)
                .collect::<Vec<Instruction>>(),
        )
        .then_ignore(just(')').padded_by(comment().repeated()).padded())
        .map(|(((name, args), locals), instructions)| Function {
            name,
            args,
            locals,
            instructions,
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
}

pub fn parse_file<'a>(file_name: &'a str) -> Option<Program> {
    let file_input = fs::read_to_string(file_name).unwrap();
    program().parse(&file_input).into_output()
}

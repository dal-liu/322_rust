use ariadne::{Color, Label, Report, ReportKind, sources};
use chumsky::prelude::*;
use l1::*;
use std::fs;

type MyExtra<'src> = extra::Err<Rich<'src, char>>;

fn separators<'src>() -> impl Parser<'src, &'src str, (), MyExtra<'src>> + Copy {
    one_of(" \t").repeated()
}

fn comment<'src>() -> impl Parser<'src, &'src str, (), MyExtra<'src>> {
    just("//").ignore_then(none_of('\n').repeated()).padded()
}

fn write_register<'src>() -> impl Parser<'src, &'src str, Register, MyExtra<'src>> {
    use Register::*;

    choice((
        arg_register(),
        just("rax").to(RAX),
        just("rbx").to(RBX),
        just("rbp").to(RBP),
        just("r10").to(R10),
        just("r11").to(R11),
        just("r12").to(R12),
        just("r13").to(R13),
        just("r14").to(R14),
        just("r15").to(R15),
    ))
    .padded_by(separators())
}

fn arg_register<'src>() -> impl Parser<'src, &'src str, Register, MyExtra<'src>> {
    use Register::*;

    choice((
        just("rdi").to(RDI),
        just("rsi").to(RSI),
        just("rdx").to(RDX),
        rcx(),
        just("r8").to(R8),
        just("r9").to(R9),
    ))
    .padded_by(separators())
}

fn rcx<'src>() -> impl Parser<'src, &'src str, Register, MyExtra<'src>> {
    just("rcx").to(Register::RCX).padded_by(separators())
}

fn value<'src>() -> impl Parser<'src, &'src str, Value, MyExtra<'src>> {
    choice((
        register_or_number(),
        function_name().map(|callee| Value::Function(callee.to_string())),
        label_name().map(|label| Value::Label(label.to_string())),
    ))
    .padded_by(separators())
}

fn register_or_number<'src>() -> impl Parser<'src, &'src str, Value, MyExtra<'src>> {
    register()
        .map(|reg| Value::Register(reg))
        .or(number().map(|num| Value::Number(num)))
        .padded_by(separators())
}

fn write_or_function<'src>() -> impl Parser<'src, &'src str, Value, MyExtra<'src>> {
    write_register()
        .map(|reg| Value::Register(reg))
        .or(function_name().map(|callee| Value::Function(callee.to_string())))
        .padded_by(separators())
}

fn register<'src>() -> impl Parser<'src, &'src str, Register, MyExtra<'src>> {
    write_register()
        .or(just("rsp").to(Register::RSP))
        .padded_by(separators())
}

fn arithmetic_op<'src>() -> impl Parser<'src, &'src str, ArithmeticOp, MyExtra<'src>> {
    choice((
        memory_arithmetic_op(),
        just("*=").to(ArithmeticOp::MultEq),
        just("&=").to(ArithmeticOp::AndEq),
    ))
    .padded_by(separators())
}

fn shift_op<'src>() -> impl Parser<'src, &'src str, ShiftOp, MyExtra<'src>> {
    choice((
        just("<<=").to(ShiftOp::LeftShiftEq),
        just(">>=").to(ShiftOp::RightShiftEq),
    ))
    .padded_by(separators())
}

fn compare_op<'src>() -> impl Parser<'src, &'src str, CompareOp, MyExtra<'src>> {
    choice((
        just("<=").to(CompareOp::LessEq),
        just("<").to(CompareOp::Less),
        just("=").to(CompareOp::Equal),
    ))
    .padded_by(separators())
}

fn multiplicative_of_8<'src>() -> impl Parser<'src, &'src str, i64, MyExtra<'src>> {
    number().filter(|n| n % 8 == 0).padded_by(separators())
}

fn number<'src>() -> impl Parser<'src, &'src str, i64, MyExtra<'src>> {
    just('+')
        .to(1)
        .or(just('-').to(-1))
        .or_not()
        .map(|opt| opt.unwrap_or(1))
        .then(text::int(10).from_str::<i128>().unwrapped())
        .map(|(sign, magnitude)| (sign * magnitude) as i64)
        .padded_by(separators())
}

fn function_name<'src>() -> impl Parser<'src, &'src str, &'src str, MyExtra<'src>> {
    just('@')
        .ignore_then(text::ascii::ident())
        .padded_by(separators())
}

fn label_name<'src>() -> impl Parser<'src, &'src str, &'src str, MyExtra<'src>> {
    just(':')
        .ignore_then(text::ascii::ident())
        .padded_by(separators())
}

fn rcx_or_number<'src>() -> impl Parser<'src, &'src str, Value, MyExtra<'src>> {
    rcx()
        .map(|reg| Value::Register(reg))
        .or(number().map(|num| Value::Number(num)))
        .padded_by(separators())
}

fn memory_arithmetic_op<'src>() -> impl Parser<'src, &'src str, ArithmeticOp, MyExtra<'src>> {
    just("+=")
        .to(ArithmeticOp::PlusEq)
        .or(just("-=").to(ArithmeticOp::MinusEq))
        .padded_by(separators())
}

fn instruction<'src>() -> impl Parser<'src, &'src str, Instruction, MyExtra<'src>> {
    let arrow = just("<-").padded_by(separators());

    let mem = just("mem").padded_by(separators());

    let call_keyword = just("call").padded_by(separators());

    let assign = write_register()
        .then_ignore(arrow)
        .then(value())
        .map(|(dst, src)| Instruction::Assign { dst, src });

    let load = write_register()
        .then_ignore(arrow.then_ignore(mem))
        .then(register())
        .then(multiplicative_of_8())
        .map(|((dst, src), offset)| Instruction::Load { dst, src, offset });

    let store = mem
        .ignore_then(register())
        .then(multiplicative_of_8())
        .then_ignore(arrow)
        .then(value())
        .map(|((dst, offset), src)| Instruction::Store { dst, offset, src });

    let arithmetic = write_register()
        .then(arithmetic_op())
        .then(register_or_number())
        .map(|((dst, op), src)| Instruction::Arithmetic { dst, op, src });

    let shift = write_register()
        .then(shift_op())
        .then(rcx_or_number())
        .map(|((dst, op), src)| Instruction::Shift { dst, op, src });

    let store_arithmetic = mem
        .ignore_then(register())
        .then(multiplicative_of_8())
        .then(memory_arithmetic_op())
        .then(register_or_number())
        .map(|(((dst, offset), op), src)| Instruction::StoreArithmetic {
            dst,
            offset,
            op,
            src,
        });

    let load_arithmetic = write_register()
        .then(memory_arithmetic_op())
        .then_ignore(mem)
        .then(register())
        .then(multiplicative_of_8())
        .map(|(((dst, op), src), offset)| Instruction::LoadArithmetic {
            dst,
            op,
            src,
            offset,
        });

    let compare = write_register()
        .then_ignore(arrow)
        .then(register_or_number())
        .then(compare_op())
        .then(register_or_number())
        .map(|(((dst, lhs), op), rhs)| Instruction::Compare { dst, lhs, op, rhs });

    let cjump = just("cjump")
        .padded_by(separators())
        .ignore_then(register_or_number())
        .then(compare_op())
        .then(register_or_number())
        .then(label_name())
        .map(|(((lhs, op), rhs), label)| Instruction::CJump {
            lhs,
            op,
            rhs,
            label: label.to_string(),
        });

    let label_inst = label_name().map(|label| Instruction::Label(label.to_string()));

    let goto = just("goto")
        .padded_by(separators())
        .ignore_then(label_name())
        .map(|label| Instruction::Goto(label.to_string()));

    let return_inst = just("return")
        .padded_by(separators())
        .to(Instruction::Return);

    let call_inst = call_keyword
        .ignore_then(write_or_function())
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

    let increment = write_register()
        .then_ignore(just("++").padded_by(separators()))
        .map(|reg| Instruction::Increment(reg));

    let decrement = write_register()
        .then_ignore(just("--").padded_by(separators()))
        .map(|reg| Instruction::Decrement(reg));

    let lea = write_register()
        .then_ignore(just('@').padded_by(separators()))
        .then(write_register())
        .then(write_register())
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

fn function<'src>() -> impl Parser<'src, &'src str, Function, MyExtra<'src>> {
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
            name: name.to_string(),
            args,
            locals,
            instructions,
        })
}

fn program<'src>() -> impl Parser<'src, &'src str, Program, MyExtra<'src>> {
    just('(')
        .padded_by(comment().repeated())
        .padded()
        .ignore_then(function_name().padded_by(comment().repeated()).padded())
        .then(function().repeated().at_least(1).collect::<Vec<Function>>())
        .then_ignore(
            just(')')
                .padded_by(comment().repeated())
                .padded()
                .then(any().repeated()),
        )
        .map(|(entry_point, functions)| Program {
            entry_point: entry_point.to_string(),
            functions,
        })
}

pub fn parse_file(file_name: &str) -> Option<Program> {
    let file_name = file_name.to_string();
    let input = fs::read_to_string(&file_name).unwrap_or_else(|e| panic!("{}", e));
    let (output, errors) = program().parse(&input).into_output_errors();

    errors.into_iter().for_each(|err| {
        Report::build(
            ReportKind::Error,
            (file_name.clone(), err.span().into_range()),
        )
        .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
        .with_message(err.to_string())
        .with_label(
            Label::new((file_name.clone(), err.span().into_range()))
                .with_message(err.reason().to_string())
                .with_color(Color::Red),
        )
        .finish()
        .eprint(sources([(file_name.clone(), input.clone())]))
        .unwrap();
    });

    output
}

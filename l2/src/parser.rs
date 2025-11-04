use ariadne::{Color, Label, Report, ReportKind, sources};
use chumsky::prelude::*;
use l2::*;
use std::{fs, mem};

macro_rules! parse {
    ( $file_name:ident, $parser:expr ) => {{
        let file_name = $file_name.to_string();
        let input = fs::read_to_string(&file_name).unwrap_or_else(|e| panic!("{}", e));

        let (output, errors) = $parser
            .parse_with_state(&input, &mut extra::SimpleState(Interner::new()))
            .into_output_errors();

        errors.into_iter().for_each(|e| {
            Report::build(
                ReportKind::Error,
                (file_name.clone(), e.span().into_range()),
            )
            .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
            .with_message(e.to_string())
            .with_label(
                Label::new((file_name.clone(), e.span().into_range()))
                    .with_message(e.reason().to_string())
                    .with_color(Color::Red),
            )
            .finish()
            .eprint(sources([(file_name.clone(), input.clone())]))
            .unwrap();
        });

        output
    }};
}

type MyExtra<'src> = extra::Full<Rich<'src, char>, extra::SimpleState<Interner<String>>, ()>;

fn separators<'src>() -> impl Parser<'src, &'src str, (), MyExtra<'src>> + Copy {
    one_of(" \t").repeated()
}

fn comment<'src>() -> impl Parser<'src, &'src str, (), MyExtra<'src>> {
    just("//").ignore_then(none_of('\n').repeated()).padded()
}

fn write_value<'src>() -> impl Parser<'src, &'src str, Value, MyExtra<'src>> {
    arg_value()
        .or(just("rax").to(Value::Register(Register::RAX)))
        .padded_by(separators())
}

fn arg_value<'src>() -> impl Parser<'src, &'src str, Value, MyExtra<'src>> {
    choice((
        just("rdi").to(Value::Register(Register::RDI)),
        just("rsi").to(Value::Register(Register::RSI)),
        just("rdx").to(Value::Register(Register::RDX)),
        rcx_or_variable(),
        just("r8").to(Value::Register(Register::R8)),
        just("r9").to(Value::Register(Register::R9)),
    ))
    .padded_by(separators())
}

fn rcx_or_variable<'src>() -> impl Parser<'src, &'src str, Value, MyExtra<'src>> {
    just("rcx")
        .to(Value::Register(Register::RCX))
        .or(variable_name()
            .map_with(|var, e| Value::Variable(SymbolId(e.state().intern(var.to_string())))))
        .padded_by(separators())
}

fn value<'src>() -> impl Parser<'src, &'src str, Value, MyExtra<'src>> {
    choice((
        register_variable_number(),
        function_name()
            .map_with(|callee, e| Value::Function(SymbolId(e.state().intern(callee.to_string())))),
        label_name()
            .map_with(|label, e| Value::Label(SymbolId(e.state().intern(label.to_string())))),
    ))
    .padded_by(separators())
}

fn register_variable_number<'src>() -> impl Parser<'src, &'src str, Value, MyExtra<'src>> {
    register_or_variable()
        .or(number().map(|num| Value::Number(num)))
        .padded_by(separators())
}

fn write_or_function<'src>() -> impl Parser<'src, &'src str, Value, MyExtra<'src>> {
    write_value()
        .or(function_name()
            .map_with(|callee, e| Value::Function(SymbolId(e.state().intern(callee.to_string())))))
        .padded_by(separators())
}

fn register_or_variable<'src>() -> impl Parser<'src, &'src str, Value, MyExtra<'src>> {
    write_value()
        .or(just("rsp").to(Value::Register(Register::RSP)))
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

fn variable_name<'src>() -> impl Parser<'src, &'src str, &'src str, MyExtra<'src>> {
    just('%')
        .ignore_then(text::ascii::ident())
        .padded_by(separators())
}

fn rcx_variable_number<'src>() -> impl Parser<'src, &'src str, Value, MyExtra<'src>> {
    rcx_or_variable()
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

    let stack_arg_keyword = just("stack-arg").padded_by(separators());

    let assign = write_value()
        .then_ignore(arrow)
        .then(value())
        .map(|(dst, src)| Instruction::Assign { dst, src });

    let load = write_value()
        .then_ignore(arrow.then_ignore(mem))
        .then(register_or_variable())
        .then(multiplicative_of_8())
        .map(|((dst, src), offset)| Instruction::Load { dst, src, offset });

    let store = mem
        .ignore_then(register_or_variable())
        .then(multiplicative_of_8())
        .then_ignore(arrow)
        .then(value())
        .map(|((dst, offset), src)| Instruction::Store { dst, offset, src });

    let stack_arg = value()
        .then_ignore(arrow.then(stack_arg_keyword))
        .then(number())
        .map(|(dst, offset)| Instruction::StackArg { dst, offset });

    let arithmetic = write_value()
        .then(arithmetic_op())
        .then(register_variable_number())
        .map(|((dst, op), src)| Instruction::Arithmetic { dst, op, src });

    let shift = write_value()
        .then(shift_op())
        .then(rcx_variable_number())
        .map(|((dst, op), src)| Instruction::Shift { dst, op, src });

    let store_arithmetic = mem
        .ignore_then(register_or_variable())
        .then(multiplicative_of_8())
        .then(memory_arithmetic_op())
        .then(register_variable_number())
        .map(|(((dst, offset), op), src)| Instruction::StoreArithmetic {
            dst,
            offset,
            op,
            src,
        });

    let load_arithmetic = write_value()
        .then(memory_arithmetic_op())
        .then_ignore(mem)
        .then(register_or_variable())
        .then(multiplicative_of_8())
        .map(|(((dst, op), src), offset)| Instruction::LoadArithmetic {
            dst,
            op,
            src,
            offset,
        });

    let compare = write_value()
        .then_ignore(arrow)
        .then(register_variable_number())
        .then(compare_op())
        .then(register_variable_number())
        .map(|(((dst, lhs), op), rhs)| Instruction::Compare { dst, lhs, op, rhs });

    let cjump = just("cjump")
        .padded_by(separators())
        .ignore_then(register_variable_number())
        .then(compare_op())
        .then(register_variable_number())
        .then(label_name())
        .map_with(|(((lhs, op), rhs), label), e| Instruction::CJump {
            lhs,
            op,
            rhs,
            label: SymbolId(e.state().intern(label.to_string())),
        });

    let label_inst = label_name()
        .map_with(|label, e| Instruction::Label(SymbolId(e.state().intern(label.to_string()))));

    let goto = just("goto")
        .padded_by(separators())
        .ignore_then(label_name())
        .map_with(|label, e| Instruction::Goto(SymbolId(e.state().intern(label.to_string()))));

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

    let increment = write_value()
        .then_ignore(just("++").padded_by(separators()))
        .map(|reg| Instruction::Increment(reg));

    let decrement = write_value()
        .then_ignore(just("--").padded_by(separators()))
        .map(|reg| Instruction::Decrement(reg));

    let lea = write_value()
        .then_ignore(just('@').padded_by(separators()))
        .then(write_value())
        .then(write_value())
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
        stack_arg,
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
        .then(
            instruction()
                .repeated()
                .at_least(1)
                .collect::<Vec<Instruction>>(),
        )
        .then_ignore(just(')').padded_by(comment().repeated()).padded())
        .map_with(|((name, args), instructions), e| {
            Function::new(
                SymbolId(e.state().intern(name.to_string())),
                args,
                instructions,
            )
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
        .map_with(|(entry_point, functions), e| Program {
            entry_point: entry_point.to_string(),
            functions,
            interner: mem::take(e.state()),
        })
}

pub fn parse_file(file_name: &str) -> Option<Program> {
    parse!(file_name, program())
}

pub fn parse_function_file(file_name: &str) -> Option<Program> {
    parse!(
        file_name,
        function().map_with(|func, e| Program {
            entry_point: "fetch".to_string(),
            functions: vec![func],
            interner: mem::take(e.state()),
        })
    )
}

pub fn parse_spill_file(file_name: &str) -> Option<(Program, Value, String)> {
    parse!(
        file_name,
        function()
            .map_with(|func, e| Program {
                entry_point: "fetch".to_string(),
                functions: vec![func],
                interner: mem::take(e.state()),
            })
            .then(variable_name().padded(),)
            .map(|(mut prog, name)| {
                let var = Value::Variable(SymbolId(prog.interner.intern(name.to_string())));
                (prog, var)
            })
            .then(variable_name().map(|var| var.to_string()).padded())
            .map(|((prog, var), prefix)| (prog, var, prefix))
    )
}

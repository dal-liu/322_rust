pub mod l1;

enum Register {
    RAX,
    RBX,
    RBP,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    RDI,
    RSI,
    RDX,
    R8,
    R9,
    RCX,
    RSP,
}

enum Value {
    Reg(Register),
    Imm(i64),
    Label(String),
}

enum ArithmeticOp {
    PlusEq,
    MinusEq,
    MultEq,
    AndEq,
}

enum ShiftOp {
    LeftShiftEq,
    RightShiftEq,
}

enum CompareOp {
    Less,
    LessEq,
    Equal,
}

enum Instruction {
    AssignInst {
        lhs: Register,
        rhs: Value,
    },
    LoadInst {
        dst: Register,
        src: Register,
        offset: i64,
    },
    StoreInst {
        dst: Register,
        offset: i64,
        src: Register,
    },
    ArithmeticInst {
        lhs: Register,
        op: ArithmeticOp,
        rhs: Value,
    },
    ShiftInst {
        lhs: Register,
        op: ShiftOp,
        rhs: Value,
    },
    StoreArithmeticInst {
        dst: Register,
        offset: i64,
        op: ArithmeticOp,
        src: Value,
    },
    LoadArithmeticInst {
        dst: Register,
        op: ArithmeticOp,
        src: Register,
        offset: i64,
    },
    CompareInst {
        dst: Register,
        lhs: Value,
        op: CompareOp,
        rhs: Value,
    },
    CJumpInst {
        lhs: Value,
        op: CompareOp,
        rhs: Value,
        label: String,
    },
    LabelInst(String),
    GotoInst(String),
    ReturnInst,
    CallInst {
        callee: Value,
        args: i64,
    },
    PrintInst,
    InputInst,
    AllocateInst,
    TupleErrorInst,
    TensorErrorInst(u8),
    IncrementInst(Register),
    DecrementInst(Register),
    LEAInst {
        dst: Register,
        src: Register,
        offset: Register,
        scale: u8,
    },
}

struct Function {
    name: String,
    args: i64,
    locals: i64,
    instructions: Vec<Instruction>,
}

struct Program {
    entryPointLabel: String,
    functions: Vec<Function>,
}

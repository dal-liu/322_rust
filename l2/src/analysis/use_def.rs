use l2::*;

pub fn uses(inst: &Instruction) -> Vec<Value> {
    use Instruction::*;
    use Register::*;

    match inst {
        Assign { src, .. } | Load { src, .. } => is_gp_variable(src)
            .then(|| vec![src.clone()])
            .unwrap_or_default(),

        Store { dst, src, .. }
        | Arithmetic { dst, src, .. }
        | Shift { dst, src, .. }
        | StoreArithmetic { dst, src, .. }
        | LoadArithmetic { dst, src, .. } => {
            let mut uses = Vec::new();
            for val in [dst, src] {
                if is_gp_variable(val) {
                    uses.push(val.clone());
                }
            }
            uses
        }

        StackArg { .. } | Label(_) | Goto(_) | Input => Vec::new(),

        Compare { lhs, rhs, .. } | CJump { lhs, rhs, .. } => {
            let mut uses = Vec::new();
            for val in [lhs, rhs] {
                if is_gp_variable(val) {
                    uses.push(val.clone());
                }
            }
            uses
        }

        Return => {
            let result_and_callee_save = [RAX, R12, R13, R14, R15, RBP, RBX];
            result_and_callee_save
                .into_iter()
                .map(Value::Register)
                .collect()
        }

        Call { callee, args } => {
            let args = *args;
            let mut uses = Vec::new();
            if is_gp_variable(callee) {
                uses.push(callee.clone());
            }
            if args >= 1 {
                uses.push(Value::Register(RDI));
            }
            if args >= 2 {
                uses.push(Value::Register(RSI));
            }
            if args >= 3 {
                uses.push(Value::Register(RDX));
            }
            if args >= 4 {
                uses.push(Value::Register(RCX));
            }
            if args >= 5 {
                uses.push(Value::Register(R8));
            }
            if args >= 6 {
                uses.push(Value::Register(R9));
            }
            uses
        }

        Print => vec![Value::Register(RDI)],

        Allocate => vec![Value::Register(RDI), Value::Register(RSI)],

        TupleError => vec![
            Value::Register(RDI),
            Value::Register(RSI),
            Value::Register(RDX),
        ],

        TensorError(args) => {
            let args = *args;
            let mut uses = Vec::new();
            if args >= 1 {
                uses.push(Value::Register(RDI));
            }
            if args >= 3 {
                uses.extend_from_slice(&[Value::Register(RSI), Value::Register(RDX)]);
            }
            if args == 4 {
                uses.push(Value::Register(RCX));
            }
            uses
        }

        Increment(val) | Decrement(val) => vec![val.clone()],

        LEA { src, offset, .. } => vec![src.clone(), offset.clone()],
    }
}

pub fn defs(inst: &Instruction) -> Vec<Value> {
    use Instruction::*;
    use Register::*;

    match inst {
        Assign { dst, .. }
        | Load { dst, .. }
        | StackArg { dst, .. }
        | Arithmetic { dst, .. }
        | Shift { dst, .. }
        | LoadArithmetic { dst, .. }
        | Compare { dst, .. }
        | LEA { dst, .. } => vec![dst.clone()],

        Store { .. } | StoreArithmetic { .. } | CJump { .. } | Label(_) | Goto(_) | Return => {
            Vec::new()
        }

        Call { .. } | Print | Input | Allocate | TupleError | TensorError(_) => {
            let caller_save = [R10, R11, R8, R9, RAX, RCX, RDI, RDX, RSI];
            caller_save.into_iter().map(Value::Register).collect()
        }

        Increment(val) | Decrement(val) => vec![val.clone()],
    }
}

fn is_gp_variable(val: &Value) -> bool {
    match val {
        Value::Variable(_) => true,
        Value::Register(reg) if !matches!(reg, Register::RSP) => true,
        _ => false,
    }
}

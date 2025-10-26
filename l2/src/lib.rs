use std::collections::HashMap;
use std::fmt;

pub const MAX_SUCCESSORS: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Register {
    RAX,
    RDI,
    RSI,
    RDX,
    R8,
    R9,
    RCX,
    RSP,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    RBP,
    RBX,
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Register::*;
        let reg = match self {
            RAX => "rax",
            RDI => "rdi",
            RSI => "rsi",
            RDX => "rdx",
            R8 => "r8",
            R9 => "r9",
            RCX => "rcx",
            RSP => "rsp",
            R10 => "r10",
            R11 => "r11",
            R12 => "r12",
            R13 => "r13",
            R14 => "r14",
            R15 => "r15",
            RBP => "rbp",
            RBX => "rbx",
        };
        write!(f, "{}", reg)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Value {
    Register(Register),
    Number(i64),
    Label(SymbolId),
    Function(SymbolId),
    Variable(SymbolId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

impl fmt::Display for SymbolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Register(r) => write!(f, "{}", r),
            Self::Number(n) => write!(f, "{}", n),
            Self::Label(s) => write!(f, ":{}", s),
            Self::Function(s) => write!(f, "@{}", s),
            Self::Variable(s) => write!(f, "%{}", s),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ArithmeticOp {
    PlusEq,
    MinusEq,
    MultEq,
    AndEq,
}

impl fmt::Display for ArithmeticOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            Self::PlusEq => "+=",
            Self::MinusEq => "-=",
            Self::MultEq => "*=",
            Self::AndEq => "&=",
        };
        write!(f, "{}", op)
    }
}

#[derive(Debug, Clone)]
pub enum ShiftOp {
    LeftShiftEq,
    RightShiftEq,
}

impl fmt::Display for ShiftOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            Self::LeftShiftEq => "<<=",
            Self::RightShiftEq => ">>=",
        };
        write!(f, "{}", op)
    }
}

#[derive(Debug, Clone)]
pub enum CompareOp {
    Less,
    LessEq,
    Equal,
}

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            Self::Less => "<",
            Self::LessEq => "<=",
            Self::Equal => "=",
        };
        write!(f, "{}", op)
    }
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Assign {
        dst: Value,
        src: Value,
    },
    Load {
        dst: Value,
        src: Value,
        offset: i64,
    },
    Store {
        dst: Value,
        offset: i64,
        src: Value,
    },
    StackArg {
        dst: Value,
        offset: i64,
    },
    Arithmetic {
        dst: Value,
        op: ArithmeticOp,
        src: Value,
    },
    Shift {
        dst: Value,
        op: ShiftOp,
        src: Value,
    },
    StoreArithmetic {
        dst: Value,
        offset: i64,
        op: ArithmeticOp,
        src: Value,
    },
    LoadArithmetic {
        dst: Value,
        op: ArithmeticOp,
        src: Value,
        offset: i64,
    },
    Compare {
        dst: Value,
        lhs: Value,
        op: CompareOp,
        rhs: Value,
    },
    CJump {
        lhs: Value,
        op: CompareOp,
        rhs: Value,
        label: SymbolId,
    },
    Label(SymbolId),
    Goto(SymbolId),
    Return,
    Call {
        callee: Value,
        args: i64,
    },
    Print,
    Input,
    Allocate,
    TupleError,
    TensorError(u8),
    Increment(Value),
    Decrement(Value),
    LEA {
        dst: Value,
        src: Value,
        offset: Value,
        scale: u8,
    },
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Instruction::*;
        match self {
            Assign { dst, src } => write!(f, "{} <- {}", dst, src),
            Load { dst, src, offset } => {
                write!(f, "{} <- mem {} {}", dst, src, offset)
            }
            Store { dst, offset, src } => {
                write!(f, "mem {} {} <- {}", dst, offset, src)
            }
            StackArg { dst, offset } => {
                write!(f, "{} <- stack-arg {}", dst, offset)
            }
            Arithmetic { dst, op, src } => write!(f, "{} {} {}", dst, op, src),
            Shift { dst, op, src } => write!(f, "{} {} {}", dst, op, src),
            StoreArithmetic {
                dst,
                offset,
                op,
                src,
            } => write!(f, "mem {} {} {} {}", dst, offset, op, src),
            LoadArithmetic {
                dst,
                op,
                src,
                offset,
            } => write!(f, "{} {} mem {} {}", dst, op, src, offset),
            Compare { dst, lhs, op, rhs } => {
                write!(f, "{} <- {} {} {}", dst, lhs, op, rhs)
            }
            CJump {
                lhs,
                op,
                rhs,
                label,
            } => write!(f, "cjump {} {} {} :{}", lhs, op, rhs, label),
            Label(label) => write!(f, ":{}", label),
            Goto(label) => write!(f, "goto :{}", label),
            Return => write!(f, "return"),
            Call { callee, args } => write!(f, "call {} {}", callee, args),
            Print => write!(f, "call print 1"),
            Input => write!(f, "call input 0"),
            Allocate => write!(f, "call allocate 2"),
            TupleError => write!(f, "call tuple-error 3"),
            TensorError(args) => write!(f, "call tensor-error {}", args),
            Increment(reg) => write!(f, "{}++", reg),
            Decrement(reg) => write!(f, "{}--", reg),
            LEA {
                dst,
                src,
                offset,
                scale,
            } => write!(f, "{} @ {} {} {}", dst, src, offset, scale),
        }
    }
}

#[derive(Debug)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
}

impl fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for inst in &self.instructions {
            writeln!(f, "\t{}", inst)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Function {
    pub name: SymbolId,
    pub args: i64,
    pub basic_blocks: Vec<BasicBlock>,
    pub cfg: ControlFlowGraph,
}

impl Function {
    pub fn build(name: SymbolId, args: i64, instructions: Vec<Instruction>) -> Self {
        let mut basic_blocks = vec![BasicBlock {
            id: BlockId(0),
            instructions: Vec::new(),
        }];

        for inst in instructions {
            let block = basic_blocks.last_mut().unwrap();
            match inst {
                Instruction::CJump { .. } | Instruction::Goto(_) | Instruction::Return => {
                    block.instructions.push(inst);
                    basic_blocks.push(BasicBlock {
                        id: BlockId(basic_blocks.len()),
                        instructions: Vec::new(),
                    });
                }
                Instruction::Label(_) => {
                    if block.instructions.is_empty() {
                        block.instructions.push(inst);
                    } else {
                        basic_blocks.push(BasicBlock {
                            id: BlockId(basic_blocks.len()),
                            instructions: vec![inst],
                        });
                    }
                }
                _ => {
                    block.instructions.push(inst);
                }
            }
        }

        if basic_blocks
            .last()
            .map_or(false, |block| block.instructions.is_empty())
        {
            basic_blocks.pop();
        }

        let cfg = ControlFlowGraph::build(&basic_blocks);

        Self {
            name,
            args,
            basic_blocks,
            cfg,
        }
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "(@{}", self.name)?;
        writeln!(f, "\t{}", self.args)?;

        for block in &self.basic_blocks {
            write!(f, "{}", block)?;
        }

        writeln!(f, ")")
    }
}

#[derive(Debug)]
pub struct ControlFlowGraph {
    pub successors: Vec<Vec<BlockId>>,
    pub predecessors: Vec<Vec<BlockId>>,
}

impl ControlFlowGraph {
    pub fn build(basic_blocks: &[BasicBlock]) -> Self {
        let label_to_block: HashMap<_, _> = basic_blocks
            .iter()
            .filter_map(|block| {
                if let Some(Instruction::Label(id)) = block.instructions.first() {
                    Some((id.clone(), block.id.clone()))
                } else {
                    None
                }
            })
            .collect();

        let num_blocks = basic_blocks.len();
        let mut cfg = Self {
            successors: vec![Vec::new(); num_blocks],
            predecessors: vec![Vec::new(); num_blocks],
        };
        let last_index = num_blocks.saturating_sub(1);

        for block in basic_blocks {
            match block.instructions.last() {
                Some(Instruction::CJump { label, .. }) => {
                    let successor = label_to_block
                        .get(&label)
                        .unwrap_or_else(|| panic!("invalid label {}", label));
                    cfg.successors[block.id.0].push(successor.clone());
                    cfg.predecessors[successor.0].push(block.id.clone());

                    if block.id.0 < last_index && block.id.0 + 1 != successor.0 {
                        cfg.successors[block.id.0].push(BlockId(block.id.0 + 1));
                        cfg.predecessors[block.id.0 + 1].push(block.id.clone());
                    }
                }
                Some(Instruction::Goto(label)) => {
                    let successor = label_to_block
                        .get(&label)
                        .unwrap_or_else(|| panic!("invalid label {}", label));
                    cfg.successors[block.id.0].push(successor.clone());
                    cfg.predecessors[successor.0].push(block.id.clone());
                }
                Some(Instruction::Return) => (),
                Some(_) => {
                    if block.id.0 < last_index {
                        cfg.successors[block.id.0].push(BlockId(block.id.0 + 1));
                        cfg.predecessors[block.id.0 + 1].push(block.id.clone());
                    }
                }
                None => panic!("empty block {}", block.id.0),
            };
        }

        cfg
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct BlockId(pub usize);

#[derive(Debug)]
pub struct Program {
    pub entry_point: SymbolId,
    pub functions: Vec<Function>,
    pub interner: Interner,
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "(@{}", self.entry_point)?;

        for func in &self.functions {
            writeln!(f, "{}", func)?;
        }

        writeln!(f, ")")
    }
}

#[derive(Debug, Default)]
pub struct Interner {
    map: HashMap<String, SymbolId>,
    vec: Vec<String>,
}

impl Interner {
    pub fn intern(&mut self, name: &str) -> SymbolId {
        if let Some(id) = self.map.get(name) {
            id.clone()
        } else {
            let id = SymbolId(self.vec.len());
            self.map.insert(name.to_string(), id.clone());
            self.vec.push(name.to_string());
            id
        }
    }

    pub fn resolve(&self, id: &SymbolId) -> &str {
        &self.vec[id.0]
    }
}

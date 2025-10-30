use l2::*;
use std::{fmt, mem};

#[derive(Debug)]
pub struct SpillDisplay<'a> {
    func: &'a Function,
}

impl fmt::Display for SpillDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "(@{}\n\t{} {}",
            self.func.interner.resolve(self.func.name.0),
            self.func.args,
            self.func.locals,
        )?;

        for block in &self.func.basic_blocks {
            write!(f, "{}", block.resolved(&self.func.interner))?;
        }

        writeln!(f, ")")
    }
}

pub fn spill_variable(func: &mut Function, var: &Value, prefix: &str) -> Vec<Value> {
    let mut modified = false;
    let mut suffix = 0;
    let mut spill_vars = Vec::new();
    let offset = func.locals * 8;

    for block in &mut func.basic_blocks {
        let len = block.instructions.len();

        for inst in mem::replace(&mut block.instructions, Vec::with_capacity(len)) {
            let spill_use = inst.uses().iter().any(|use_| use_ == var);
            let spill_def = inst.defs().iter().any(|def| def == var);

            let spill_var = if spill_use || spill_def {
                let new_var = Value::Variable(SymbolId(
                    func.interner.intern(format!("{}{}", prefix, suffix)),
                ));
                modified = true;
                suffix += 1;
                spill_vars.push(new_var.clone());
                Some(new_var)
            } else {
                None
            };

            if spill_use {
                if let Some(ref new_var) = spill_var {
                    block.instructions.push(Instruction::Load {
                        dst: new_var.clone(),
                        src: Value::Register(Register::RSP),
                        offset,
                    });
                }
            }

            if spill_use || spill_def {
                let mut new_inst = inst.clone();
                if let Some(ref new_var) = spill_var {
                    new_inst.replace_value(var, new_var);
                }
                block.instructions.push(new_inst);
            } else {
                block.instructions.push(inst);
            }

            if spill_def {
                if let Some(ref new_var) = spill_var {
                    block.instructions.push(Instruction::Store {
                        dst: Value::Register(Register::RSP),
                        offset,
                        src: new_var.clone(),
                    });
                }
            }
        }
    }

    if modified {
        func.locals += 1;
    }

    spill_vars
}

pub fn spill_variable_with_display<'a>(
    func: &'a mut Function,
    var: &'a Value,
    prefix: &'a str,
) -> SpillDisplay<'a> {
    spill_variable(func, var, prefix);
    SpillDisplay { func }
}

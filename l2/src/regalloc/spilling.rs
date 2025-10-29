use l2::*;
use std::fmt;

#[derive(Debug)]
pub struct SpillDisplay<'a> {
    func: &'a Function,
}

impl fmt::Display for SpillDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "(@{}\n\t{} {}",
            self.func.name.resolved(&self.func.interner),
            self.func.args,
            self.func.locals,
        )?;

        for block in &self.func.basic_blocks {
            write!(f, "{}", block.resolved(&self.func.interner))?;
        }

        writeln!(f, ")")
    }
}

pub fn spill_variable(func: &mut Function, var: &Value, prefix: &str) {
    let mut spilled = false;
    let mut suffix = 0;
    let offset = func.locals * 8;

    for block in &mut func.basic_blocks {
        let mut new_instructions = Vec::with_capacity(block.instructions.len());

        for inst in &block.instructions {
            let spill_use = inst.uses().iter().any(|use_| use_ == var);
            let spill_def = inst.defs().iter().any(|def| def == var);

            let spill_var = if spill_use || spill_def {
                spilled = true;
                let name = &format!("{}{}", prefix, suffix);
                suffix += 1;
                Some(Value::Variable(func.interner.intern(name)))
            } else {
                None
            };

            if spill_use {
                if let Some(ref spill_var) = spill_var {
                    new_instructions.push(Instruction::Load {
                        dst: spill_var.clone(),
                        src: Value::Register(Register::RSP),
                        offset,
                    });
                }
            }

            let mut new_inst = inst.clone();
            if let Some(ref spill_var) = spill_var {
                new_inst.replace_value(var, spill_var);
            }
            new_instructions.push(new_inst);

            if spill_def {
                if let Some(ref spill_var) = spill_var {
                    new_instructions.push(Instruction::Store {
                        dst: Value::Register(Register::RSP),
                        offset,
                        src: spill_var.clone(),
                    });
                }
            }
        }

        block.instructions = new_instructions;
    }

    if spilled {
        func.locals += 1;
    }
}

pub fn spill_variable_with_display<'a>(
    func: &'a mut Function,
    var: &'a Value,
    prefix: &'a str,
) -> SpillDisplay<'a> {
    spill_variable(func, var, prefix);
    SpillDisplay { func }
}

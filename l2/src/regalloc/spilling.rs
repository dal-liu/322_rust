use l2::*;
use std::collections::HashMap;
use std::{fmt, mem};

#[derive(Debug)]
pub struct SpillDisplay<'a> {
    func: &'a Function,
    interner: &'a Interner<String>,
}

impl fmt::Display for SpillDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "(@{}\n\t{} {}",
            self.interner.resolve(self.func.name.0),
            self.func.args,
            self.func.locals,
        )?;

        for block in &self.func.basic_blocks {
            write!(f, "{}", block.resolved(&self.interner))?;
        }

        writeln!(f, ")")
    }
}

pub fn spill(
    func: &mut Function,
    var: &Value,
    prefix: &str,
    suffix: &mut i32,
    interner: &mut Interner<String>,
) -> Vec<Value> {
    let mut modified = false;
    let mut spill_vars = Vec::new();
    let offset = func.locals * 8;

    for block in &mut func.basic_blocks {
        let num_insts = block.instructions.len();
        for inst in mem::replace(&mut block.instructions, Vec::with_capacity(num_insts)) {
            let spill_use = inst.uses().iter().any(|use_| use_ == var);
            let spill_def = inst.defs().iter().any(|def| def == var);

            let spill_var = if spill_use || spill_def {
                let new_var =
                    Value::Variable(SymbolId(interner.intern(format!("{}{}", prefix, suffix))));
                modified = true;
                *suffix += 1;
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

pub fn spill_with_display<'a>(
    func: &'a mut Function,
    var: &Value,
    prefix: &str,
    interner: &'a mut Interner<String>,
) -> SpillDisplay<'a> {
    let mut suffix = 0;
    spill(func, var, prefix, &mut suffix, interner);
    SpillDisplay { func, interner }
}

pub fn spill_all(
    func: &mut Function,
    prefix: &str,
    suffix: &mut i32,
    interner: &mut Interner<String>,
) {
    let mut var_to_offset = HashMap::new();

    for block in &mut func.basic_blocks {
        let num_insts = block.instructions.len();
        for inst in mem::replace(&mut block.instructions, Vec::with_capacity(num_insts)) {
            let uses = inst.uses();
            let defs = inst.defs();
            let mut var_to_spill = HashMap::new();

            for use_ in &uses {
                if matches!(use_, Value::Variable(_)) && !var_to_spill.contains_key(use_) {
                    let new_var =
                        Value::Variable(SymbolId(interner.intern(format!("{}{}", prefix, suffix))));
                    *suffix += 1;
                    var_to_spill.insert(use_, new_var);
                }
            }

            for def in &defs {
                if matches!(def, Value::Variable(_)) && !var_to_spill.contains_key(def) {
                    let new_var =
                        Value::Variable(SymbolId(interner.intern(format!("{}{}", prefix, suffix))));
                    *suffix += 1;
                    var_to_spill.insert(def, new_var);
                }
            }

            for use_ in &uses {
                if let Some(new_var) = var_to_spill.get(use_) {
                    let offset = *var_to_offset.entry(use_.clone()).or_insert_with(|| {
                        let offset = func.locals * 8;
                        func.locals += 1;
                        offset
                    });
                    block.instructions.push(Instruction::Load {
                        dst: new_var.clone(),
                        src: Value::Register(Register::RSP),
                        offset,
                    })
                }
            }

            let mut new_inst = inst.clone();
            for (&old_var, new_var) in &var_to_spill {
                new_inst.replace_value(old_var, new_var);
            }
            block.instructions.push(new_inst);

            for def in &defs {
                if let Some(new_var) = var_to_spill.get(def) {
                    let offset = *var_to_offset.entry(def.clone()).or_insert_with(|| {
                        let offset = func.locals * 8;
                        func.locals += 1;
                        offset
                    });
                    block.instructions.push(Instruction::Store {
                        dst: Value::Register(Register::RSP),
                        offset: offset,
                        src: new_var.clone(),
                    })
                }
            }
        }
    }
}

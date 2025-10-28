use crate::analysis::def_use::{defs, uses};
use l2::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ValueInterner {
    map: HashMap<Value, usize>,
    vec: Vec<Value>,
}

impl ValueInterner {
    pub fn build(func: &Function) -> Self {
        let mut value_map = Self {
            map: HashMap::new(),
            vec: Vec::new(),
        };

        for block in &func.basic_blocks {
            for inst in &block.instructions {
                for use_ in uses(inst) {
                    value_map.intern(&use_);
                }
                for def in defs(inst) {
                    value_map.intern(&def);
                }
            }
        }

        value_map
    }

    pub fn intern(&mut self, value: &Value) -> usize {
        if let Some(&idx) = self.map.get(value) {
            idx
        } else {
            let idx = self.vec.len();
            self.map.insert(value.clone(), idx);
            self.vec.push(value.clone());
            idx
        }
    }

    pub fn resolve(&self, index: usize) -> &Value {
        &self.vec[index]
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }
}

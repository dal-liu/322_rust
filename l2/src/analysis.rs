mod bitvector;
mod liveness;
mod use_def;
mod value_interner;
mod worklist;

pub use liveness::compute_liveness;

use crate::analysis::value_interner::ValueInterner;
use l2::*;
use std::collections::HashSet;

#[derive(Debug)]
pub struct AnalysisResult {
    pub gen_: Vec<Vec<HashSet<Value>>>,
    pub kill: Vec<Vec<HashSet<Value>>>,
    pub in_: Vec<Vec<HashSet<Value>>>,
    pub out: Vec<Vec<HashSet<Value>>>,
}

impl DisplayResolved for AnalysisResult {
    fn fmt_with(&self, f: &mut std::fmt::Formatter, interner: &StringInterner) -> std::fmt::Result {
        writeln!(f, "(\n(in")?;

        for vec in &self.in_ {
            for set in vec {
                let mut line = set
                    .iter()
                    .map(|val| format!("{}", val.resolved(interner)))
                    .collect::<Vec<_>>();
                line.sort();
                writeln!(f, "({})", line.join(" "))?;
            }
        }

        writeln!(f, ")\n\n(out")?;

        for vec in &self.out {
            for set in vec {
                let mut line = set
                    .iter()
                    .map(|val| format!("{}", val.resolved(interner)))
                    .collect::<Vec<_>>();
                line.sort();
                writeln!(f, "({})", line.join(" "))?;
            }
        }

        writeln!(f, ")\n\n)")
    }
}

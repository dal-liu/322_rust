use crate::analysis::LivenessResult;
use crate::bitvector::BitVector;

use l2::*;

#[derive(Debug)]
pub struct InterferenceGraph<'a> {
    pub interner: &'a Interner<Value>,
    pub graph: Vec<BitVector>,
}

impl<'a> InterferenceGraph<'a> {
    pub fn new(func: &Function, liveness: &'a LivenessResult) -> Self {
        let num_gp_variables = liveness.interner.len();
        let mut graph = Self {
            interner: &liveness.interner,
            graph: vec![BitVector::new(num_gp_variables); num_gp_variables],
        };

        let gp_registers: Vec<usize> = Register::GP_REGISTERS
            .iter()
            .map(|&reg| liveness.interner.get(&Value::Register(reg)).unwrap())
            .collect();
        for &u in &gp_registers {
            for &v in &gp_registers {
                if u < v {
                    graph.add_edge(u, v);
                }
            }
        }

        for block in &func.basic_blocks {
            let mut live = liveness.out[block.id.0].clone();

            for inst in block.instructions.iter().rev() {
                match inst {
                    Instruction::Assign { src, .. } if src.is_gp_variable() => {
                        live.reset(liveness.interner.get(src).unwrap());
                    }
                    Instruction::Shift { src, .. } if matches!(src, Value::Variable(_)) => {
                        let rcx = graph.interner.get(&Value::Register(Register::RCX)).unwrap();
                        let u = graph.interner.get(src).unwrap();
                        for &v in &gp_registers {
                            if v != rcx {
                                graph.add_edge(u, v);
                            }
                        }
                    }
                    _ => (),
                }

                let defs: Vec<usize> = inst
                    .defs()
                    .iter()
                    .map(|def| liveness.interner.get(def).unwrap())
                    .collect();

                live.set_from(defs.iter().copied());
                for &u in &defs {
                    for v in &live {
                        if u != v {
                            graph.add_edge(u, v);
                        }
                    }
                }

                live.reset_from(defs.iter().copied());
                live.set_from(
                    inst.uses()
                        .iter()
                        .map(|use_| liveness.interner.get(use_).unwrap()),
                );
            }
        }

        graph
    }

    pub fn add_edge(&mut self, u: usize, v: usize) {
        self.graph[u].set(v);
        self.graph[v].set(u);
    }

    pub fn has_edge(&self, u: usize, v: usize) -> bool {
        self.graph[u].test(v)
    }

    pub fn degree(&self, node: usize) -> u32 {
        self.graph[node].count()
    }
}

pub fn build_interference<'a>(
    func: &Function,
    liveness: &'a LivenessResult,
) -> InterferenceGraph<'a> {
    InterferenceGraph::new(func, liveness)
}

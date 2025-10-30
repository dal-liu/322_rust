use crate::analysis::LivenessResult;
use crate::bitvector::BitVector;

use l2::*;
use std::fmt;

#[derive(Debug)]
pub struct InterferenceGraph {
    interner: Interner<Value>,
    graph: Vec<BitVector>,
}

impl InterferenceGraph {
    pub fn build(func: &Function, liveness: &LivenessResult) -> Self {
        use Register::*;

        let mut interner = liveness.interner.clone();
        let gp_registers = [
            RAX, RDI, RSI, RDX, R8, R9, RCX, R10, R11, R12, R13, R14, R15, RBP, RBX,
        ]
        .map(|reg| interner.intern(Value::Register(reg)));

        let num_values = interner.len();
        let mut graph = Self {
            interner,
            graph: vec![BitVector::with_capacity(num_values); num_values],
        };

        for u in gp_registers {
            for v in gp_registers {
                if u < v {
                    graph.add_edge(u, v);
                }
            }
        }

        for block in &func.basic_blocks {
            let i = block.id.0;

            for (j, inst) in block.instructions.iter().enumerate() {
                let in_ = &liveness.in_[i][j];
                for u in in_.iter() {
                    for v in in_.iter() {
                        if u < v {
                            graph.add_edge(u, v);
                        }
                    }
                }

                let out = &liveness.out[i][j];
                for u in out.iter() {
                    for v in out.iter() {
                        if u < v {
                            graph.add_edge(u, v);
                        }
                    }
                }

                let kill = &liveness.kill[i][j];
                for u in kill.iter() {
                    for v in out.iter() {
                        if u != v {
                            graph.add_edge(u, v);
                        }
                    }
                }

                if let Instruction::Shift { src, .. } = inst {
                    if matches!(src, Value::Variable(_)) {
                        let rcx = graph
                            .interner
                            .get(&Value::Register(RCX))
                            .unwrap_or_else(|| panic!("rcx not interned"));
                        let u = graph
                            .interner
                            .get(src)
                            .unwrap_or_else(|| panic!("{:?} not interned", src));
                        for v in gp_registers {
                            if v != rcx {
                                graph.add_edge(u, v);
                            }
                        }
                    }
                }
            }
        }

        graph
    }

    pub fn add_edge(&mut self, u: usize, v: usize) {
        self.graph[u].set(v);
        self.graph[v].set(u);
    }

    pub fn remove_node(&mut self, node: usize) {
        let neighbors: Vec<usize> = self.graph[node].iter().collect();
        for neighbor in neighbors {
            self.graph[neighbor].reset(node);
        }
        self.graph[node].clear();
    }
}

impl DisplayResolved for InterferenceGraph {
    fn fmt_with(&self, f: &mut fmt::Formatter, interner: &Interner<String>) -> fmt::Result {
        let mut lines: Vec<String> = (0..self.graph.len())
            .into_iter()
            .map(|i| {
                let mut line: Vec<String> = self.graph[i]
                    .iter()
                    .map(|j| self.interner.resolve(j).resolved(interner).to_string())
                    .collect();
                line.sort();
                format!(
                    "{} {}",
                    self.interner.resolve(i).resolved(interner),
                    line.join(" ")
                )
            })
            .collect();
        lines.sort();
        writeln!(f, "{}", lines.join("\n"))
    }
}

pub fn compute_interference(func: &Function, liveness: &LivenessResult) -> InterferenceGraph {
    InterferenceGraph::build(func, liveness)
}

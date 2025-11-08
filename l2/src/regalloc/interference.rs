use crate::analysis::LivenessResult;
use crate::bitvector::BitVector;

use l2::*;
use std::fmt;

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
            .map(|&reg| {
                liveness
                    .interner
                    .get(&Value::Register(reg))
                    .expect("registers should be interned")
            })
            .collect();

        for &u in &gp_registers {
            for &v in &gp_registers {
                if u < v {
                    graph.add_edge(u, v);
                }
            }
        }

        for block in &func.basic_blocks {
            let mut live = liveness.block_out[block.id.0].clone();

            for inst in block.instructions.iter().rev() {
                let defs: Vec<usize> = inst
                    .defs()
                    .iter()
                    .map(|def| liveness.interner.get(def).expect("defs should be interned"))
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

                live.set_from(inst.uses().iter().map(|use_| {
                    liveness
                        .interner
                        .get(use_)
                        .expect("uses should be interned")
                }));

                match inst {
                    Instruction::Shift { src, .. } if matches!(src, Value::Variable(_)) => {
                        let rcx = graph
                            .interner
                            .get(&Value::Register(Register::RCX))
                            .expect("rcx not interned");

                        let u = graph
                            .interner
                            .get(src)
                            .expect("variables should be interned");

                        for &v in &gp_registers {
                            if v != rcx {
                                graph.add_edge(u, v);
                            }
                        }
                    }
                    _ => (),
                }
            }

            // let i = block.id.0;
            //
            // for (j, inst) in block.instructions.iter().enumerate() {
            //     let in_ = &liveness.in_[i][j];
            //     for u in in_ {
            //         for v in in_ {
            //             if u < v {
            //                 graph.add_edge(u, v);
            //             }
            //         }
            //     }
            //
            //     let out = &liveness.out[i][j];
            //     for u in out {
            //         for v in out {
            //             if u < v {
            //                 graph.add_edge(u, v);
            //             }
            //         }
            //     }
            //
            //     let kill: Vec<usize> = inst
            //         .uses()
            //         .iter()
            //         .map(|use_| {
            //             liveness
            //                 .interner
            //                 .get(use_)
            //                 .expect("uses should be interned")
            //         })
            //         .collect();
            //
            //     for u in kill {
            //         for v in out {
            //             if u != v {
            //                 graph.add_edge(u, v);
            //             }
            //         }
            //     }
            //
            //     if let Instruction::Shift { src, .. } = inst {
            //         if matches!(src, Value::Variable(_)) {
            //             let rcx = graph
            //                 .interner
            //                 .get(&Value::Register(Register::RCX))
            //                 .expect("rcx not interned");
            //
            //             let u = graph
            //                 .interner
            //                 .get(src)
            //                 .expect("variables should be interned");
            //
            //             for &v in &gp_registers {
            //                 if v != rcx {
            //                     graph.add_edge(u, v);
            //                 }
            //             }
            //         }
            //     }
            // }
        }

        graph
    }

    pub fn add_edge(&mut self, u: usize, v: usize) {
        self.graph[u].set(v);
        self.graph[v].set(u);
    }
}

impl DisplayResolved for InterferenceGraph<'_> {
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

impl fmt::Display for InterferenceGraph<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut lines: Vec<String> = (0..self.graph.len())
            .into_iter()
            .map(|i| {
                let mut line: Vec<String> = self.graph[i].iter().map(|j| j.to_string()).collect();
                line.sort();
                format!("{} {}", i, line.join(" "))
            })
            .collect();
        lines.sort();
        writeln!(f, "{}", lines.join("\n"))
    }
}

pub fn build_interference<'a>(
    func: &Function,
    liveness: &'a LivenessResult,
) -> InterferenceGraph<'a> {
    InterferenceGraph::new(func, liveness)
}

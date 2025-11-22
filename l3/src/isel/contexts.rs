use l3::*;

#[derive(Debug)]
pub struct Context<'a> {
    pub instructions: Vec<&'a Instruction>,
}

pub fn create_contexts<'a>(func: &'a Function) -> Vec<Context<'a>> {
    let mut contexts = vec![Context {
        instructions: Vec::new(),
    }];

    func.basic_blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .for_each(|inst| {
            let context = contexts.last_mut().unwrap();

            match inst {
                Instruction::Return
                | Instruction::ReturnValue(_)
                | Instruction::Branch(_)
                | Instruction::BranchCond { .. } => {
                    context.instructions.push(inst);
                    contexts.push(Context {
                        instructions: Vec::new(),
                    });
                }

                Instruction::Label(_)
                | Instruction::Call { .. }
                | Instruction::CallResult { .. } => contexts.push(Context {
                    instructions: Vec::new(),
                }),

                _ => context.instructions.push(inst),
            }
        });

    contexts.retain(|ctx| !ctx.instructions.is_empty());

    contexts
}

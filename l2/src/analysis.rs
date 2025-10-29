mod liveness;
mod value_interner;
mod worklist;

pub use crate::analysis::liveness::{LivenessResult, compute_liveness};
pub use crate::analysis::value_interner::ValueInterner;

mod coloring;
mod interference;
mod spilling;

pub use coloring::color_graph;
pub use interference::compute_interference;
pub use spilling::spill_variable_with_display;

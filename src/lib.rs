use std::sync::{LazyLock, OnceLock, RwLock};

use raug::prelude::*;

pub mod plugin;
pub mod processor;

pub static ENGINE: LazyLock<rhai::Engine> = LazyLock::new(|| init_engine(rhai::Engine::new()));
pub static AST: OnceLock<rhai::AST> = OnceLock::new();
pub static GRAPH: LazyLock<Graph> = LazyLock::new(Graph::new);
pub static STREAM: RwLock<Option<CpalStream>> = RwLock::new(None);

pub fn init_engine(mut engine: rhai::Engine) -> rhai::Engine {
    engine.set_strict_variables(true);
    engine.set_max_expr_depths(0, 0);
    engine.set_optimization_level(rhai::OptimizationLevel::Simple);

    plugin::init_engine(&mut engine);

    engine
}

pub mod prelude {
    pub use crate::processor::RhaiProcessor;
}

use std::{
    path::PathBuf,
    sync::{LazyLock, Mutex, OnceLock},
};

use clap::Parser;
use raug::{graph::Graph, prelude::CpalStream};
use rhai::{Engine, exported_module};

pub mod plugin;
pub mod processor;

pub static ENGINE: LazyLock<Engine> = LazyLock::new(init_engine);
pub static AST: OnceLock<rhai::AST> = OnceLock::new();
pub static GRAPH: LazyLock<Graph> = LazyLock::new(Graph::new);
pub static STREAM: Mutex<Option<CpalStream>> = Mutex::new(None);

fn init_engine() -> Engine {
    let mut engine = Engine::new();
    let plugin_module = exported_module!(plugin::raug_plugin);
    engine.set_strict_variables(true);
    engine.set_max_expr_depths(0, 0);
    engine.build_type::<plugin::raug_plugin::RhaiNode>();
    engine.build_type::<plugin::raug_plugin::RhaiOutput>();
    engine.build_type::<plugin::raug_plugin::RhaiInput>();
    engine.build_type::<plugin::raug_plugin::RhaiGraph>();
    engine.register_global_module(plugin_module.into());
    engine
}

#[derive(Parser)]
struct Args {
    file: PathBuf,
}

fn main() {
    let args = Args::parse();
    let file = args.file;
    let source = std::fs::read_to_string(file).unwrap();
    let ast = ENGINE.compile(source).unwrap();
    AST.get_or_init(|| ast);
    ENGINE
        .call_fn::<()>(&mut rhai::Scope::default(), AST.get().unwrap(), "main", ())
        .unwrap();
}

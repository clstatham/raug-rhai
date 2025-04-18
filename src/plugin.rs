use raug::{
    graph::node::{IntoInputIdx, IntoOutputIdx},
    prelude::*,
};

use raug_ext::prelude::*;

use rhai::plugin::*;

use crate::processor::{RhaiProcEnv, RhaiProcessorInternal};

pub(crate) fn init_engine(engine: &mut rhai::Engine) {
    engine
        .build_type::<RhaiNode>()
        .build_type::<RhaiInput>()
        .build_type::<RhaiOutput>()
        .build_type::<RhaiProcEnv>();
    engine.register_global_module(exported_module!(raug_plugin).into());
}

#[derive(Clone)]
struct RhaiDynamic(rhai::Dynamic);

impl From<rhai::Dynamic> for RhaiDynamic {
    fn from(value: rhai::Dynamic) -> Self {
        RhaiDynamic(value)
    }
}

impl IntoOutput for RhaiDynamic {
    fn into_output(self, graph: &Graph) -> Output {
        if let Some(node) = self.0.clone().try_cast::<RhaiNode>() {
            node.0.assert_single_output("RhaiDynamic::into_output()");
            node.0.output(0)
        } else if let Some(node) = self.0.clone().try_cast::<f32>() {
            node.into_output(graph)
        } else if let Some(node) = self.0.clone().try_cast::<i64>() {
            node.into_output(graph)
        } else if let Some(node) = self.0.clone().try_cast::<bool>() {
            node.into_output(graph)
        } else if let Some(node) = self.0.clone().try_cast::<RhaiOutput>() {
            node.0
        } else {
            panic!("Invalid type for IntoOutput: {}", self.0.type_name());
        }
    }
}

impl IntoInputIdx for RhaiDynamic {
    fn into_input_idx(self, node: &Node) -> u32 {
        if let Ok(idx) = self.0.as_int() {
            (idx as u32).into_input_idx(node)
        } else if let Ok(idx) = self.0.as_immutable_string_ref() {
            idx.into_input_idx(node)
        } else {
            panic!("Invalid type for IntoInputIdx: {}", self.0.type_name());
        }
    }
}

impl IntoOutputIdx for RhaiDynamic {
    fn into_output_idx(self, node: &Node) -> u32 {
        if let Ok(idx) = self.0.as_int() {
            (idx as u32).into_output_idx(node)
        } else if let Ok(idx) = self.0.as_immutable_string_ref() {
            idx.into_output_idx(node)
        } else {
            panic!("Invalid type for IntoOutputIdx: {}", self.0.type_name());
        }
    }
}

#[derive(Clone)]
pub struct RhaiOutput(Output);

impl rhai::CustomType for RhaiOutput {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("Output");
        builder.with_fn("connect", |output: &mut Self, input: RhaiInput| {
            output.0.connect(&input.0);
            output.clone()
        });
        builder.with_fn("node", |output: &mut Self| RhaiNode(output.0.node()));
    }
}

#[derive(Clone)]
pub struct RhaiInput(Input);

impl rhai::CustomType for RhaiInput {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("Input");
        builder.with_fn("connect", |input: &mut Self, output: rhai::Dynamic| {
            input.0.connect(RhaiDynamic(output));
            input.clone()
        });
        builder.with_fn("node", |input: &mut Self| RhaiNode(input.0.node()));
    }
}

#[derive(Clone)]
pub struct RhaiNode(Node);

impl rhai::CustomType for RhaiNode {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("Node");
        builder.with_fn("input", |node: &mut Self, index: rhai::Dynamic| {
            RhaiInput(node.0.input(RhaiDynamic(index).into_input_idx(&node.0)))
        });
        builder.with_fn("output", |node: &mut Self, index: rhai::Dynamic| {
            RhaiOutput(node.0.output(RhaiDynamic(index).into_output_idx(&node.0)))
        });
        builder.with_fn("set_inputs", |node: &mut Self, idxs: rhai::Array| {
            for (i, input) in idxs.into_iter().enumerate() {
                node.0.input(i as u32).connect(RhaiDynamic(input));
            }
            node.clone()
        });
    }
}

#[rhai::export_module(name = "raug")]
mod raug_plugin {
    use super::*;

    #[rhai_fn(global)]
    pub fn readln() {
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read from stdin");
    }

    #[rhai_fn(global)]
    pub fn dac(input: RhaiOutput) -> RhaiNode {
        let node = crate::GRAPH.add_audio_output();
        node.input(0).connect(input.0);
        RhaiNode(node)
    }

    #[rhai_fn(global)]
    pub fn adc() -> RhaiOutput {
        let node = crate::GRAPH.add_audio_input();
        RhaiOutput(node.output(0))
    }

    #[rhai_fn(global)]
    pub fn processor(map: rhai::Map) -> RhaiNode {
        let node = crate::GRAPH.add(RhaiProcessorInternal::from_map(map));
        RhaiNode(node)
    }

    #[rhai_fn(global)]
    pub fn sine_osc() -> RhaiNode {
        let node = crate::GRAPH.add(SineOscillator::default());
        RhaiNode(node)
    }
}

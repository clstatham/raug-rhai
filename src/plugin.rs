use raug::prelude::*;
use raug_ext::prelude::*;
use rhai::{CustomType, TypeBuilder, plugin::*};

#[export_module(name = "raug")]
pub mod raug_plugin {
    use crate::{GRAPH, processor::RhaiFnProcessor};

    use super::*;

    #[derive(Clone, CustomType)]
    #[rhai_type(name = "Output")]
    pub struct RhaiOutput(Output);

    #[derive(Clone, CustomType)]
    #[rhai_type(name = "Input")]
    pub struct RhaiInput(Input);

    #[derive(Clone, CustomType)]
    #[rhai_type(name = "Graph")]
    pub struct RhaiGraph(Graph);

    #[derive(Clone, CustomType)]
    #[rhai_type(name = "Node")]
    pub struct RhaiNode(Node);

    #[rhai_fn(global)]
    pub fn inputs(node: RhaiNode, inputs: rhai::Array) -> RhaiNode {
        for (i, input) in inputs.into_iter().enumerate() {
            let input = if let Ok(input) = input.as_float() {
                RhaiOutput(input.into_output(&GRAPH))
            } else if let Ok(input) = input.as_int() {
                RhaiOutput(input.into_output(&GRAPH))
            } else if let Ok(input) = input.as_bool() {
                RhaiOutput(input.into_output(&GRAPH))
            } else if let Ok(input) = input.clone().try_cast_result::<RhaiOutput>() {
                input
            } else {
                panic!("Invalid input type for node: {}", input.type_name());
            };
            node.0.input(i as u32).connect(input.0);
        }
        node.clone()
    }

    #[rhai_fn(global)]
    pub fn output(node: RhaiNode, index: Dynamic) -> RhaiOutput {
        if let Ok(index) = index.as_int() {
            RhaiOutput(node.0.output(index as u32))
        } else if let Ok(index) = index.as_immutable_string_ref() {
            RhaiOutput(node.0.output(&*index.to_string()))
        } else {
            panic!("Invalid index type for output: {}", index.type_name());
        }
    }

    #[rhai_fn(global)]
    pub fn play() {
        if crate::STREAM.lock().unwrap().is_some() {
            return;
        }
        let mut stream = CpalStream::default();
        stream.spawn(&crate::GRAPH).unwrap();
        stream.play().unwrap();
        crate::STREAM.lock().unwrap().replace(stream);
    }

    #[rhai_fn(global)]
    pub fn stop() {
        if let Some(mut stream) = crate::STREAM.lock().unwrap().take() {
            stream.stop().unwrap();
            stream.join().unwrap();
        }
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
    pub fn sample_rate() -> RhaiOutput {
        let node = crate::GRAPH.add(SampleRate::default());
        RhaiOutput(node.output(0))
    }

    #[rhai_fn(global)]
    pub fn processor(map: rhai::Map) -> RhaiNode {
        let node = crate::GRAPH.add(RhaiFnProcessor::from_map(map));
        RhaiNode(node)
    }

    #[rhai_fn(global)]
    pub fn sine_osc() -> RhaiNode {
        let node = crate::GRAPH.add(SineOscillator::default());
        RhaiNode(node)
    }
}

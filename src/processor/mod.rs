use raug::{prelude::*, processor::io::ProcessorOutput};

use crate::{AST, ENGINE};

fn str_to_signal_type(s: &str) -> SignalType {
    match s {
        "f32" => f32::signal_type(),
        "i32" => i32::signal_type(),
        "i64" => i64::signal_type(),
        "u32" => u32::signal_type(),
        "u64" => u64::signal_type(),
        "bool" => bool::signal_type(),
        _ => panic!("Unknown signal type: {}", s),
    }
}

fn buf_of_type(size: usize, signal_type: SignalType) -> AnyBuffer {
    match signal_type {
        t if t == f32::signal_type() => AnyBuffer::zeros::<f32>(size),
        t if t == i32::signal_type() => AnyBuffer::zeros::<i32>(size),
        t if t == i64::signal_type() => AnyBuffer::zeros::<i64>(size),
        t if t == u32::signal_type() => AnyBuffer::zeros::<u32>(size),
        t if t == u64::signal_type() => AnyBuffer::zeros::<u64>(size),
        t if t == bool::signal_type() => AnyBuffer::zeros::<bool>(size),
        _ => panic!("Unknown signal type: {:?}", signal_type),
    }
}

fn dynamic_of_type(signal: &AnySignalRef, signal_type: SignalType) -> rhai::Dynamic {
    match signal_type {
        t if t == f32::signal_type() => rhai::Dynamic::from(*signal.downcast_ref::<f32>().unwrap()),
        t if t == i32::signal_type() => rhai::Dynamic::from(*signal.downcast_ref::<i32>().unwrap()),
        t if t == i64::signal_type() => rhai::Dynamic::from(*signal.downcast_ref::<i64>().unwrap()),
        t if t == u32::signal_type() => rhai::Dynamic::from(*signal.downcast_ref::<u32>().unwrap()),
        t if t == u64::signal_type() => rhai::Dynamic::from(*signal.downcast_ref::<u64>().unwrap()),
        t if t == bool::signal_type() => {
            rhai::Dynamic::from(*signal.downcast_ref::<bool>().unwrap())
        }
        _ => panic!("Unknown signal type: {:?}", signal_type),
    }
}

fn set_from_dynamic(
    output: &mut ProcessorOutput,
    sample_index: usize,
    signal_type: SignalType,
    value: &rhai::Dynamic,
) {
    match signal_type {
        t if t == f32::signal_type() => {
            let value = value.as_float().unwrap();
            *output.get_mut_as(sample_index).unwrap() = value;
        }
        t if t == i32::signal_type() => {
            let value = value.as_int().unwrap() as i32;
            *output.get_mut_as(sample_index).unwrap() = value;
        }
        t if t == i64::signal_type() => {
            let value = value.as_int().unwrap();
            *output.get_mut_as(sample_index).unwrap() = value;
        }
        t if t == u32::signal_type() => {
            let value = value.as_int().unwrap() as u32;
            *output.get_mut_as(sample_index).unwrap() = value;
        }
        t if t == u64::signal_type() => {
            let value = value.as_int().unwrap() as u64;
            *output.get_mut_as(sample_index).unwrap() = value;
        }
        t if t == bool::signal_type() => {
            let value = value.as_bool().unwrap();
            *output.get_mut_as(sample_index).unwrap() = value;
        }
        _ => panic!("Unknown signal type: {:?}", signal_type),
    }
}

struct InputArgs(smallvec::SmallVec<[rhai::Dynamic; 16]>);

impl rhai::FuncArgs for InputArgs {
    fn parse<ARGS: Extend<rhai::Dynamic>>(self, args: &mut ARGS) {
        for arg in self.0 {
            args.extend(std::iter::once(arg));
        }
    }
}

pub struct RhaiFnProcessor {
    map: rhai::Dynamic,
    input_spec: Vec<SignalSpec>,
    output_spec: Vec<SignalSpec>,
    fn_name: String,
}

impl RhaiFnProcessor {
    pub fn from_map(map: rhai::Map) -> Self {
        let process_func = map
            .get("process")
            .expect("No `process` function found in the script");

        let process_func = process_func
            .flatten_clone()
            .try_cast_result::<rhai::FnPtr>()
            .expect("Failed to cast `process` function to `FnPtr`");
        let fn_name = process_func.fn_name().to_string();

        let mut input_spec = Vec::new();
        let mut output_spec = Vec::new();

        let input_spec_value = map
            .get("input_spec")
            .expect("No `input_spec` found in the script");
        let output_spec_value = map
            .get("output_spec")
            .expect("No `output_spec` found in the script");

        let input_spec_list = input_spec_value
            .as_array_ref()
            .expect("Expected `input_spec` to be an array")
            .clone();
        for value in input_spec_list.iter() {
            let value = value.as_array_ref().expect("Expected value to be an array");

            let [name, signal_type] = &(*value)[..] else {
                panic!("Expected value to be an array of length 2")
            };

            let name = name
                .as_immutable_string_ref()
                .expect("Expected name to be a string");

            let signal_type = str_to_signal_type(
                signal_type
                    .as_immutable_string_ref()
                    .as_deref()
                    .expect("Expected signal type to be a string"),
            );

            let signal_spec = SignalSpec::new(name.to_string(), signal_type);
            input_spec.push(signal_spec);
        }

        let output_spec_list = output_spec_value
            .as_array_ref()
            .expect("Expected `output_spec` to be an array")
            .clone();
        for value in output_spec_list.iter() {
            let value = value.as_array_ref().expect("Expected value to be an array");

            let [name, signal_type] = &(*value)[..] else {
                panic!("Expected value to be an array of length 2")
            };

            let name = name
                .as_immutable_string_ref()
                .expect("Expected name to be a string");

            let signal_type = str_to_signal_type(
                signal_type
                    .as_immutable_string_ref()
                    .as_deref()
                    .expect("Expected signal type to be a string"),
            );

            let signal_spec = SignalSpec::new(name.to_string(), signal_type);
            output_spec.push(signal_spec);
        }

        Self {
            map: map.into(),
            input_spec,
            output_spec,
            fn_name,
        }
    }
}

impl Processor for RhaiFnProcessor {
    fn input_spec(&self) -> Vec<SignalSpec> {
        self.input_spec.clone()
    }

    fn output_spec(&self) -> Vec<SignalSpec> {
        self.output_spec.clone()
    }

    fn create_output_buffers(&self, size: usize) -> Vec<AnyBuffer> {
        let mut output_buffers = Vec::new();
        for spec in &self.output_spec {
            output_buffers.push(buf_of_type(size, spec.signal_type));
        }
        output_buffers
    }

    fn process(
        &mut self,
        inputs: ProcessorInputs,
        mut outputs: ProcessorOutputs,
    ) -> Result<(), ProcessorError> {
        let mut input_values = smallvec::SmallVec::<[rhai::Dynamic; 16]>::new();
        for sample_index in 0..inputs.block_size() {
            for (input_index, input_spec) in self.input_spec.iter().enumerate() {
                let input_signal = inputs
                    .input(input_index)
                    .unwrap()
                    .get(sample_index)
                    .unwrap();
                let input_value = dynamic_of_type(&input_signal, input_spec.signal_type);
                input_values.push(input_value);
            }

            let result: rhai::Array = ENGINE
                .call_fn_with_options(
                    rhai::CallFnOptions::new()
                        .bind_this_ptr(&mut self.map)
                        .eval_ast(true),
                    &mut rhai::Scope::new(),
                    AST.get().unwrap(),
                    &self.fn_name,
                    InputArgs(input_values.clone()),
                )
                .expect("Failed to call `process` function");

            for (output_index, output_spec) in self.output_spec.iter().enumerate() {
                let output_value = &result[output_index];
                set_from_dynamic(
                    &mut outputs.output(output_index),
                    sample_index,
                    output_spec.signal_type,
                    output_value,
                );
            }

            input_values.clear();
        }

        Ok(())
    }
}

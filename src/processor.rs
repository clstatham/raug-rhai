use raug::{prelude::*, processor::io::ProcessorOutput};

use crate::ENGINE;

fn str_to_signal_type(s: &str) -> SignalType {
    match s {
        "f32" => f32::signal_type(),
        "i64" => i64::signal_type(),
        "bool" => bool::signal_type(),
        _ => panic!("Unknown signal type: {}", s),
    }
}

fn buf_of_type(size: usize, signal_type: SignalType) -> AnyBuffer {
    match signal_type {
        t if t == f32::signal_type() => AnyBuffer::zeros::<f32>(size),
        t if t == i64::signal_type() => AnyBuffer::zeros::<i64>(size),
        t if t == bool::signal_type() => AnyBuffer::zeros::<bool>(size),
        _ => panic!("Unknown signal type: {:?}", signal_type),
    }
}

fn dynamic_of_type(signal: &AnySignalRef, signal_type: SignalType) -> rhai::Dynamic {
    match signal_type {
        t if t == f32::signal_type() => rhai::Dynamic::from(*signal.downcast_ref::<f32>().unwrap()),
        t if t == i64::signal_type() => rhai::Dynamic::from(*signal.downcast_ref::<i64>().unwrap()),
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
        t if t == i64::signal_type() => {
            let value = value.as_int().unwrap();
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

#[derive(Debug, Clone)]
pub struct RhaiProcEnv(ProcEnv);

impl rhai::CustomType for RhaiProcEnv {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("ProcEnv")
            .with_get("sample_rate", |this: &mut Self| this.0.sample_rate)
            .with_get("block_size", |this: &mut Self| this.0.block_size);
    }
}

pub(crate) struct RhaiProcessorInternal {
    map: rhai::Dynamic,
    input_spec: Vec<SignalSpec>,
    output_spec: Vec<SignalSpec>,
    fn_name: String,
    scope: rhai::Scope<'static>,
}

impl RhaiProcessorInternal {
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

        let scope = rhai::Scope::new();

        Self {
            map: map.into(),
            input_spec,
            output_spec,
            fn_name,
            scope,
        }
    }

    pub fn process_internal(
        &mut self,
        inputs: ProcessorInputs,
        outputs: &mut ProcessorOutputs,
        engine: &rhai::Engine,
        ast: &rhai::AST,
    ) -> Result<(), ProcessorError> {
        let mut input_args = smallvec::SmallVec::<[rhai::Dynamic; 16]>::new();
        input_args.reserve(self.input_spec.len() + 1);

        for sample_index in 0..inputs.block_size() {
            input_args.push(rhai::Dynamic::from(RhaiProcEnv(inputs.env)));

            for (input_index, input_spec) in self.input_spec.iter().enumerate() {
                let input_signal = inputs
                    .input(input_index)
                    .unwrap()
                    .get(sample_index)
                    .unwrap();
                let input_arg = dynamic_of_type(&input_signal, input_spec.signal_type);
                input_args.push(input_arg);
            }

            let result: rhai::Array = engine
                .call_fn_with_options(
                    rhai::CallFnOptions::new()
                        .bind_this_ptr(&mut self.map)
                        .eval_ast(true),
                    &mut self.scope,
                    ast,
                    &self.fn_name,
                    InputArgs(input_args.clone()),
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

            input_args.clear();
        }

        Ok(())
    }
}

impl Processor for RhaiProcessorInternal {
    fn name(&self) -> &str {
        "RhaiProcessor"
    }

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
        self.process_internal(
            inputs,
            &mut outputs,
            &crate::ENGINE,
            crate::AST.get().unwrap(),
        )?;

        Ok(())
    }
}

pub struct RhaiProcessor {
    ast: rhai::AST,
    inner: RhaiProcessorInternal,
}

impl RhaiProcessor {
    pub fn new(script: &str) -> Self {
        let ast = ENGINE.compile(script).unwrap();
        let map = ENGINE.eval_ast::<rhai::Map>(&ast).unwrap();

        Self {
            ast,
            inner: RhaiProcessorInternal::from_map(map),
        }
    }
}

impl Processor for RhaiProcessor {
    fn name(&self) -> &str {
        "RhaiProcessor"
    }

    fn input_spec(&self) -> Vec<SignalSpec> {
        self.inner.input_spec()
    }

    fn output_spec(&self) -> Vec<SignalSpec> {
        self.inner.output_spec()
    }

    fn create_output_buffers(&self, size: usize) -> Vec<AnyBuffer> {
        self.inner.create_output_buffers(size)
    }

    fn process(
        &mut self,
        inputs: ProcessorInputs,
        mut outputs: ProcessorOutputs,
    ) -> Result<(), ProcessorError> {
        self.inner
            .process_internal(inputs, &mut outputs, &ENGINE, &self.ast)?;

        Ok(())
    }
}

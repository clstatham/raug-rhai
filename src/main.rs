use std::{path::PathBuf, sync::LazyLock};

use clap::Parser;
use raug::prelude::*;
use raug_rhai::{AST, ENGINE, GRAPH, STREAM};

#[derive(Parser)]
struct Args {
    file: PathBuf,
    #[clap(long, default_value = "default")]
    backend: String,
    #[clap(long, default_value = "default")]
    device: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let source = std::fs::read_to_string(&args.file)?;

    LazyLock::force(&ENGINE);
    LazyLock::force(&GRAPH);

    let mut scope = rhai::Scope::new();

    let ast = ENGINE.compile_with_scope(&scope, source)?;
    AST.get_or_init(|| ast);

    let backend = match args.backend.to_lowercase().as_str() {
        "default" => AudioBackend::default(),
        #[cfg(target_os = "linux")]
        "alsa" => AudioBackend::Alsa,
        #[cfg(target_os = "linux")]
        "jack" => AudioBackend::Jack,
        #[cfg(target_os = "windows")]
        "wasapi" => AudioBackend::Wasapi,
        _ => {
            return Err(anyhow::anyhow!("Unsupported backend: {}", args.backend));
        }
    };

    let device = if args.device.as_str() == "default" {
        AudioDevice::default()
    } else if let Ok(index) = args.device.parse::<usize>() {
        AudioDevice::Index(index)
    } else {
        AudioDevice::Name(args.device.clone())
    };

    let mut stream = CpalStream::new(backend, device);
    stream.spawn(&GRAPH)?;
    stream.play()?;
    STREAM.write().unwrap().replace(stream);

    ENGINE.call_fn::<()>(&mut scope, AST.get().unwrap(), "main", ())?;

    let mut stream = STREAM.write().unwrap().take().unwrap();
    stream.stop()?;
    stream.join()?;
    Ok(())
}

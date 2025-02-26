mod vm;

use std::{
    fs, io,
    path::PathBuf,
    time::{Duration, Instant},
};

use clap::Parser;
use floretta::Autodiff;
use wasmtime::{Engine, Instance, Module, Store};

#[derive(Parser)]
struct Cli {
    input: PathBuf,
}

fn main() {
    let args = Cli::parse();
    let mut timer = Timer::new();

    let vm = if args.input.to_str() == Some("-") {
        io::read_to_string(io::stdin()).unwrap()
    } else {
        fs::read_to_string(args.input).unwrap()
    };
    println!("read into memory: {:?}", timer.tick());

    let wasm = vm::to_wasm(&vm);
    println!("convert to Wasm: {:?}", timer.tick());

    let mut ad = Autodiff::no_validate();
    ad.export("main", "backprop");
    let grad = ad.reverse(&wasm).unwrap();
    println!("autodiff: {:?}", timer.tick());

    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, &grad).unwrap();
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let main = instance
        .get_typed_func::<(f32, f32), f32>(&mut store, "main")
        .unwrap();
    let backprop = instance
        .get_typed_func::<f32, (f32, f32)>(&mut store, "backprop")
        .unwrap();
    println!("compile: {:?}", timer.tick());

    main.call(&mut store, (0., 0.)).unwrap();
    backprop.call(&mut store, 1.).unwrap();
    println!("run: {:?}", timer.tick());
}

struct Timer {
    previous: Instant,
}

impl Timer {
    fn new() -> Self {
        Self {
            previous: Instant::now(),
        }
    }

    fn tick(&mut self) -> Duration {
        let now = Instant::now();
        let duration = now - self.previous;
        self.previous = now;
        duration
    }
}

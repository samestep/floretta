use std::{
    borrow::Cow,
    fs,
    io::{self, IsTerminal, Read, Write},
    path::PathBuf,
};

use anyhow::bail;
use clap::Parser;
use floretta::Autodiff;
use itertools::Itertools;
use termcolor::{ColorChoice, NoColor, StandardStream, WriteColor};

/// Apply automatic differentiation to a WebAssembly module.
#[derive(Debug, Parser)]
#[command(name = "floretta", version)]
struct Cli {
    /// Input file path, or `-` to read from stdin.
    input: PathBuf,

    /// Forward mode.
    #[clap(short, long)]
    forward: bool,

    /// Reverse mode.
    #[clap(short, long)]
    reverse: bool,

    /// Do not validate the input WebAssembly module.
    #[clap(long)]
    no_validate: bool,

    /// Do not include the name section in the output WebAssembly module.
    #[clap(long)]
    no_names: bool,

    /// In reverse mode, import the backward pass of a function that is already imported.
    #[clap(short, long, value_names=["MODULE", "NAME", "MODULE", "NAME"])]
    import: Vec<String>,

    /// In the output Wasm, also export the derivative counterpart of an export from the input Wasm.
    #[clap(short, long, value_names=["NAME", "NAME"])]
    export: Vec<String>,

    /// Output file path; if not provided, will write to stdout.
    #[clap(short, long)]
    output: Option<PathBuf>,

    /// Output the WebAssembly text format instead of the binary format.
    #[clap(short = 't', long)]
    wat: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let raw = if args.input.to_str() == Some("-") {
        let mut stdin = Vec::new();
        io::stdin().read_to_end(&mut stdin)?;
        stdin
    } else {
        fs::read(args.input)?
    };
    let before = match wat::parse_bytes(&raw)? {
        Cow::Borrowed(bytes) => {
            assert_eq!((bytes.as_ptr(), bytes.len()), (raw.as_ptr(), raw.len()));
            raw
        }
        Cow::Owned(bytes) => bytes,
    };
    let mut ad = if args.no_validate {
        Autodiff::no_validate()
    } else {
        Autodiff::new()
    };
    if !args.no_names {
        ad.names();
    }
    for quadruple in args.import.into_iter().chunks(4).into_iter() {
        let (fwd_module, fwd_name, bwd_module, bwd_name) = quadruple.collect_tuple().unwrap();
        ad.import((fwd_module, fwd_name), (bwd_module, bwd_name));
    }
    for pair in args.export.into_iter().chunks(2).into_iter() {
        let (forward, backward) = pair.collect_tuple().unwrap();
        ad.export(forward, backward);
    }
    let after = match (args.forward, args.reverse) {
        (false, false) => bail!("must select either `--forward` mode or `--reverse` mode"),
        (true, true) => bail!("can't select both forward mode and reverse mode at once"),
        (true, false) => ad.forward(&before)?,
        (false, true) => ad.reverse(&before)?,
    };
    if args.wat {
        match args.output {
            Some(path) => {
                let writer = NoColor::new(io::BufWriter::new(fs::File::create(path)?));
                print_wat(&after, writer)?;
            }
            None => {
                let color = if io::stdout().is_terminal() {
                    ColorChoice::Auto
                } else {
                    ColorChoice::Never
                };
                print_wat(&after, StandardStream::stdout(color))?;
            }
        }
    } else {
        match args.output {
            Some(path) => fs::write(path, after)?,
            None => {
                let mut stdout = std::io::stdout();
                if stdout.is_terminal() {
                    bail!("can't print binary to terminal; redirect or give `--output` or `--wat`");
                }
                stdout.write_all(&after)?;
            }
        }
    }
    Ok(())
}

fn print_wat(wasm: &[u8], writer: impl WriteColor) -> anyhow::Result<()> {
    wasmprinter::Config::new().print(wasm, &mut wasmprinter::PrintTermcolor(writer))?;
    Ok(())
}

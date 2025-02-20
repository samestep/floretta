use std::{
    fs,
    io::{self, IsTerminal, Read, Write},
    path::PathBuf,
};

use anyhow::bail;
use clap::Parser;
use floretta::Autodiff;

/// Apply reverse-mode automatic differentiation to a WebAssembly module.
#[derive(Debug, Parser)]
struct Cli {
    /// Input file path, or `-` to read from stdin.
    input: PathBuf,

    /// Do not validate the input WebAssembly module.
    #[clap(long)]
    no_validate: bool,

    /// Do not include the name section in the output WebAssembly module.
    #[clap(long)]
    no_names: bool,

    /// Export the backward pass of a function that is already exported.
    #[clap(long = "export", num_args = 2)]
    name: Vec<String>,

    /// Output file path; if not provided, will write to stdout.
    #[clap(short, long)]
    output: Option<PathBuf>,

    /// Output the WebAssembly text format instead of the binary format.
    #[clap(short = 't', long)]
    wat: bool,
}

pub fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let before = if args.input.to_str() == Some("-") {
        let mut stdin = Vec::new();
        io::stdin().read_to_end(&mut stdin)?;
        wat::parse_bytes(&stdin)?.into_owned()
    } else {
        wat::parse_file(args.input)?
    };
    let mut ad = if args.no_validate {
        Autodiff::no_validate()
    } else {
        Autodiff::new()
    };
    if !args.no_names {
        ad.names();
    }
    for pair in args.name.chunks(2) {
        ad.export(&pair[0], &pair[1]);
    }
    let after = ad.transform(&before)?;
    if args.wat {
        let string = wasmprinter::print_bytes(after)?;
        match args.output {
            Some(path) => fs::write(path, string)?,
            None => print!("{}", string),
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

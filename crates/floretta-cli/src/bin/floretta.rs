use std::{
    fs,
    io::{self, IsTerminal, Read, Write},
    path::PathBuf,
};

use anyhow::bail;
use clap::Parser;

/// Apply reverse-mode automatic differentiation to a WebAssembly module.
#[derive(Debug, Parser)]
struct Cli {
    /// Input file path; if not provided, will read from stdin.
    input: Option<PathBuf>,

    /// Output file path; if not provided, will write to stdout.
    #[clap(short, long)]
    output: Option<PathBuf>,

    /// Output the WebAssembly text format instead of the binary format.
    #[clap(short = 't', long)]
    wat: bool,
}

pub fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let before = match args.input {
        Some(path) => wat::parse_file(path)?,
        None => {
            let mut stdin = Vec::new();
            io::stdin().read_to_end(&mut stdin)?;
            wat::parse_bytes(&stdin)?.into_owned()
        }
    };
    let after = floretta::autodiff(&before);
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

mod archive_format;
mod cli;
mod commands;

use std::process;

use cli::*;

fn main() -> anyhow::Result<()> {
    let args = get_args();

    let code = match &args.command {
        CLICommands::A {
            ..
        } => commands::handle_compression(args)?,
        CLICommands::X {
            ..
        } => commands::handle_decompression(args)?,
    };

    process::exit(code.unwrap_or(1));
}

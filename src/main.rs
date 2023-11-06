mod archive_format;
mod cli;
mod commands;

use cli::*;

fn main() -> anyhow::Result<()> {
    let args = get_args();

    match &args.command {
        CLICommands::A {
            ..
        } => {
            commands::handle_compression(args)?;
        },
        CLICommands::X {
            ..
        } => {
            commands::handle_decompression(args)?;
        },
    }

    Ok(())
}

use std::env;
use std::process::exit;

pub enum CliArgs {
    NoArguments,
    Help,
    CleanHistory,
    Unknown
}

fn cli_args_parser() -> anyhow::Result<CliArgs> {
    let args: Vec<String> = env::args().collect();
    let arg_len = args.len();

    if arg_len > 1 {
        match args[1].as_str() {
            "--help" | "-h" => Ok(CliArgs::Help),
            "--clean-history" | "-c" => Ok(CliArgs::CleanHistory),
            _ => Ok(CliArgs::Unknown)
        }
    } else {
        Ok(CliArgs::NoArguments)
    }
}

pub fn cli_args_handler() {
    let cli_args: CliArgs = cli_args_parser().unwrap();
    match cli_args {
        CliArgs::NoArguments => {}, // Nothing, continue as usual
        CliArgs::Help => {
            println!("ClipVault - A secure clipboard manager");
            println!();
            println!("Usage:");
            println!("  clipvault -h or --help    Show this help message");
            println!("  clipvault -c or --clean-history     Clean the clipboard history");
            println!();
            println!("Hotkey:");
            println!("  Super + V                 Toggle clipboard history window");
            exit(0);
        }
        CliArgs::CleanHistory => {
            println!("Cleaning clipboard history...");
            // TODO
            exit(0);
        }
        CliArgs::Unknown => {
            println!("Unknown argument");
            exit(1);
        }
    }
}
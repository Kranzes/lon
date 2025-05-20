mod bot;
mod cli;
mod commit_message;
mod config;
mod git;
mod http;
mod lock;
mod lon_nix;
mod nix;
mod sources;

use std::process::ExitCode;

use cli::Cli;

fn main() -> ExitCode {
    Cli::init(module_path!())
}

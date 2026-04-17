use std::process::ExitCode;

#[path = "../main.rs"]
mod bin_main;

fn main() -> ExitCode {
    bin_main::main()
}

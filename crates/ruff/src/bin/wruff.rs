use std::io::Write;
use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;
use colored::Colorize;

use ruff::args::Args;
use ruff::{ExitStatus, run};

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "openbsd"),
    not(target_os = "aix"),
    not(target_os = "android"),
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "powerpc64",
        target_arch = "riscv64"
    )
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() -> ExitCode {
    #[cfg(windows)]
    assert!(colored::control::set_virtual_terminal(true).is_ok());

    let args = wild::args_os();
    let args = match argfile::expand_args_from(args, argfile::parse_fromfile, argfile::PREFIX)
        .context("Failed to read CLI arguments from files")
    {
        Ok(args) => args,
        Err(err) => return report_error(&err),
    };

    let args = match Args::parse_from(args).apply_public_aliases() {
        Ok(args) => args,
        Err(err) => return report_error(&err),
    };

    match run(args) {
        Ok(code) => code.into(),
        Err(err) => report_error(&err),
    }
}

fn report_error(err: &anyhow::Error) -> ExitCode {
    for cause in err.chain() {
        if let Some(ioerr) = cause.downcast_ref::<std::io::Error>()
            && ioerr.kind() == std::io::ErrorKind::BrokenPipe
        {
            return ExitCode::from(0);
        }
    }

    let mut stderr = std::io::stderr().lock();
    writeln!(stderr, "{}", "ruff failed".red().bold()).ok();
    for cause in err.chain() {
        writeln!(stderr, "  {} {cause}", "Cause:".bold()).ok();
    }
    ExitStatus::Error.into()
}

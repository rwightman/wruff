use std::io::Write;
use std::path::Path;
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

pub(crate) fn main() -> ExitCode {
    // Enabled ANSI colors on Windows 10.
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
    {
        // Exit "gracefully" on broken pipe errors.
        //
        // See: https://github.com/BurntSushi/ripgrep/blob/bf63fe8f258afc09bae6caa48f0ae35eaf115005/crates/core/main.rs#L47C1-L61C14
        for cause in err.chain() {
            if let Some(ioerr) = cause.downcast_ref::<std::io::Error>() {
                if ioerr.kind() == std::io::ErrorKind::BrokenPipe {
                    return ExitCode::from(0);
                }
            }
        }

        // Use `writeln` instead of `eprintln` to avoid panicking when the stderr pipe is broken.
        let mut stderr = std::io::stderr().lock();

        // This communicates that this isn't a linter error but ruff itself hard-errored for
        // some reason (e.g. failed to resolve the configuration)
        let executable_name = std::env::args_os()
            .next()
            .as_deref()
            .and_then(|path| Path::new(path).file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("wruff")
            .to_owned();
        writeln!(
            stderr,
            "{}",
            format!("{executable_name} failed").red().bold()
        )
        .ok();
        // Currently we generally only see one error, but e.g. with io errors when resolving
        // the configuration it is help to chain errors ("resolving configuration failed" ->
        // "failed to read file: subdir/pyproject.toml")
        for cause in err.chain() {
            writeln!(stderr, "  {} {cause}", "Cause:".bold()).ok();
        }
    }
    ExitStatus::Error.into()
}

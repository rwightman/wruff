use std::io::{self, BufWriter, Write};
use std::path::Path;

use anyhow::Result;

use crate::args::HelpFormat;

/// Display version information
pub(crate) fn version(output_format: HelpFormat) -> Result<()> {
    let mut stdout = BufWriter::new(io::stdout().lock());
    let version_info = crate::version::version();

    match output_format {
        HelpFormat::Text => {
            let executable_name = std::env::args_os()
                .next()
                .as_deref()
                .and_then(|path| Path::new(path).file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("wruff")
                .to_owned();
            writeln!(stdout, "{executable_name} {}", &version_info)?;
        }
        HelpFormat::Json => {
            serde_json::to_writer_pretty(stdout, &version_info)?;
        }
    }
    Ok(())
}

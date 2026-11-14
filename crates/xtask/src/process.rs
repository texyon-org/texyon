//! Running the tools a build drives.

use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

/// Run `command` with inherited output, failing unless it exits successfully.
pub fn run(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("could not start {}", describe(command)))?;
    if !status.success() {
        bail!("{} failed with {status}", describe(command));
    }
    Ok(())
}

/// Run `command` and return its standard output.
pub fn stdout(command: &mut Command) -> Result<String> {
    let output = command
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("could not start {}", describe(command)))?;
    if !output.status.success() {
        bail!("{} failed with {}", describe(command), output.status);
    }
    String::from_utf8(output.stdout)
        .with_context(|| format!("{} printed non-UTF-8", describe(command)))
}

fn describe(command: &Command) -> String {
    let mut line = command.get_program().to_string_lossy().into_owned();
    for arg in command.get_args() {
        line.push(' ');
        line.push_str(&arg.to_string_lossy());
    }
    line
}

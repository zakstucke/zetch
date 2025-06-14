use std::{
    io::Read,
    path::Path,
    process::{ChildStderr, ChildStdout},
};

use crate::prelude::*;

pub struct CmdOut<'a> {
    commands: &'a [String],
    stdouts: Vec<String>,
    stderrs: Vec<String>,
}

impl<'a> CmdOut<'a> {
    pub fn new(
        commands: &'a [String],
        child_stdouts: Vec<ChildStdout>,
        child_stderrs: Vec<ChildStderr>,
    ) -> Result<Self, Report<Zerr>> {
        let mut stdouts = vec![];
        let mut stderrs = vec![];

        for mut child_stdout in child_stdouts {
            let mut output = String::new();
            child_stdout
                .read_to_string(&mut output)
                .change_context(Zerr::InternalError)
                .attach_printable("Failed to read stdout")?;
            stdouts.push(output);
        }

        for mut child_stderr in child_stderrs {
            let mut output = String::new();
            child_stderr
                .read_to_string(&mut output)
                .change_context(Zerr::InternalError)
                .attach_printable("Failed to read stderr")?;
            stderrs.push(output);
        }

        Ok(Self {
            commands,
            stdouts,
            stderrs,
        })
    }

    pub fn iter_formatted_commands_and_outputs(&self) -> impl Iterator<Item = String> {
        self.commands
            .iter()
            .zip(self.stdouts.iter().zip(self.stderrs.iter()))
            .map(|(command, (stdout, stderr))| {
                format!("Command: '{command}'\nStdout:\n{stdout}\nStderr:\n{stderr}")
            })
    }

    pub fn last_stdout(&self) -> &str {
        self.stdouts.last().map_or("", String::as_str)
    }
}

// Run the commands return the final stdout:
pub fn run_cmd<'a>(
    config_dir: &Path,
    commands: &'a [String],
    envvars: &[(&str, String)],
) -> Result<CmdOut<'a>, Report<Zerr>> {
    let mut child_stdouts = vec![];
    let mut child_stderrs = vec![];
    for command in commands {
        let result = (|| {
            let args = shell_words::split(command)
                .change_context(Zerr::InternalError)
                .attach_printable_lazy(|| format!("Failed to split command: '{command}'"))?;
            let first = if let Some(first) = args.first() {
                first
            } else {
                return Ok(()); // Skip empty commands
            };
            // TODO once tokio add kill_on_drop as a fallback.
            let mut cmd = std::process::Command::new(first);
            cmd.current_dir(config_dir)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            for (key, value) in envvars {
                cmd.env(key, value);
            }

            for arg in &args[1..] {
                cmd.arg(arg);
            }

            let mut child = cmd
                .spawn()
                .change_context(Zerr::UserCommandError)
                .attach_printable("Failed with non zero exit code")?;
            child_stdouts.push(child.stdout.take().ok_or_else(|| {
                zerr!(
                    Zerr::InternalError,
                    "Failed to capture stdout for command: {}",
                    command
                )
            })?);
            child_stderrs.push(child.stderr.take().ok_or_else(|| {
                zerr!(
                    Zerr::InternalError,
                    "Failed to capture stderr for command: {}",
                    command
                )
            })?);

            let status = child.wait().change_context(Zerr::InternalError)?;
            if !status.success() {
                return Err(zerr!(
                    Zerr::UserCommandError,
                    "Failed with non zero exit code, status: {}",
                    status
                ));
            }

            Ok(())
        })()
        .attach_printable_lazy(|| format!("Command: '{command}' failed"));

        // Attach the run command outputs:
        if let Err(mut e) = result {
            let cmd_out = CmdOut::new(commands, child_stdouts, child_stderrs)?;
            for output in cmd_out.iter_formatted_commands_and_outputs() {
                e = e.attach_printable(output);
            }
            return Err(e);
        }
    }

    CmdOut::new(commands, child_stdouts, child_stderrs)
}

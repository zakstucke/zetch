use std::path::Path;

use bitbazaar::{
    cli::{Bash, BashErr},
    timeit,
};
use serde::{Deserialize, Serialize};

use super::{
    parent_config::{store_parent_config, CACHED_CONFIG_ENV_VAR},
    Config,
};
use crate::prelude::*;

static IN_TASK_ENV_VAR: &str = "ZETCH_IN_TASK";

pub fn parent_task_active() -> bool {
    std::env::var(IN_TASK_ENV_VAR).is_ok()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Task {
    pub commands: Vec<String>,
}

impl Task {
    /// Run the task, post tasks will be given the env var to the post ctx path.
    fn run(&self, config_filepath: &Path, cached_config_loc: Option<&Path>) -> Result<(), Zerr> {
        // Make sure no recursion:
        if std::env::var(IN_TASK_ENV_VAR).is_ok() {
            return Err(zerr!(
                Zerr::TaskRecursionError,
                "Tasks being run recursively. Make sure you're not running a zetch command that triggers tasks from inside tasks.\nE.g. 'zetch render'.\n\nHint: 'zetch render|var' commands in 'pre' tasks won't work with due to recursive task constraints,\n       however, 'zetch var' does work in 'post' tasks thanks to some internal magic."
            ));
        }

        let pre_or_post_str = if cached_config_loc.is_none() {
            "pre"
        } else {
            "post"
        };

        let config_dir = config_filepath.parent().ok_or_else(|| {
            zerr!(
                Zerr::InternalError,
                "Failed to get parent dir of config file: {}",
                config_filepath.display()
            )
        })?;

        // Create the bash environment:
        let mut bash = Bash::new().chdir(config_dir);
        bash = bash.env(IN_TASK_ENV_VAR, "1");
        if let Some(cached_config_loc) = cached_config_loc {
            bash = bash.env(
                CACHED_CONFIG_ENV_VAR,
                cached_config_loc.display().to_string(),
            );
        }

        for command in self.commands.iter() {
            bash = bash.cmd(command);
        }

        let cmd_out = match timeit!(format!("Cmd ({})", pre_or_post_str).as_str(), {
            bash.run()
        }) {
            Ok(cmd_out) => Ok(cmd_out),
            Err(e) => match e.current_context() {
                BashErr::InternalError(_) => Err(e.change_context(Zerr::InternalError)),
                _ => Err(e.change_context(Zerr::UserCommandError)),
            },
        }?;
        cmd_out.throw_on_bad_code(Zerr::UserCommandError)?;

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Tasks {
    #[serde(default = "Vec::new")]
    pub pre: Vec<Task>,
    #[serde(default = "Vec::new")]
    pub post: Vec<Task>,
}

impl Tasks {
    /// Run the pre tasks that are given no special environment:
    pub fn run_pre(&self, config_filepath: &Path) -> Result<(), Zerr> {
        for task in self.pre.iter() {
            task.run(config_filepath, None)?;
        }
        Ok(())
    }

    pub fn run_post(&self, conf: &Config) -> Result<(), Zerr> {
        // Will cache the config so subcommands using it will work.
        let path_buf = store_parent_config(conf)?;
        let path = path_buf.as_path();

        for task in self.post.iter() {
            task.run(&conf.final_config_path, Some(path))?;
        }

        Ok(())
    }
}

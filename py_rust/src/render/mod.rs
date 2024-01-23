use bitbazaar::{
    timeit,
    timing::{format_duration, GLOBAL_TIME_RECORDER},
};
use colored::Colorize;
use minijinja::context;
use tracing::{debug, info, warn};

mod args_validate;
mod debug;
mod lockfile;
mod template;
mod walker;
pub use lockfile::hash_contents;
pub use walker::get_template_matcher_rewrite_mapping;

use crate::{
    args::RenderCommand,
    config::{self, final_config_path, Config},
    prelude::*,
};

pub fn render(args: &crate::args::Args, render_args: &RenderCommand) -> Result<bool, Zerr> {
    args_validate::args_validate(render_args)?;

    let raw_conf = timeit!("Config processing", {
        config::RawConfig::from_toml(&final_config_path(&args.config, Some(&render_args.root))?)
    })?;

    let mut lockfile = timeit!("Lockfile preparation", {
        self::lockfile::Lockfile::load(render_args.root.clone(), render_args.force)
    });

    let (conf, written, identical) = if lockfile.newly_created
        && raw_conf.context.cli.values().any(|v| v.initial.is_some())
    {
        warn!("Lockfile newly created/force updated and some cli vars have initials, will double render and use initials first time round.");
        // Conf from second as that has the real cli vars, template info from the first as the second will be inaccurate due to the first having run.
        let (_, first_written, first_identical) =
            render_inner(args, render_args, raw_conf.clone(), &mut lockfile, true)?;
        let (conf, _, _) = render_inner(args, render_args, raw_conf, &mut lockfile, false)?;
        (conf, first_written, first_identical)
    } else {
        render_inner(args, render_args, raw_conf, &mut lockfile, false)?
    };

    timeit!("Syncing lockfile", { lockfile.sync() })?;

    // Write only when hidden cli flag --debug is set, to allow testing internals from python without having to setup custom interfaces:
    if render_args.debug {
        let debug = debug::Debug {
            config: conf,
            written: written
                .iter()
                .map(|t| t.out_path.display().to_string())
                .collect(),
            identical: identical.iter().map(|t| t.rel_path.clone()).collect(),
            lockfile_modified: lockfile.modified,
        };

        // Write as json to zetch_debug.json at root:
        let debug_json =
            serde_json::to_string_pretty(&debug).change_context(Zerr::InternalError)?;
        std::fs::write(render_args.root.join("zetch_debug.json"), debug_json)
            .change_context(Zerr::InternalError)?;
    }

    info!(
        "{} {} template{} written, {} identical. Lockfile {}. {} elapsed.",
        "zetch:".bold(),
        written.len(),
        if written.len() == 1 { "" } else { "s" },
        identical.len(),
        if lockfile.modified {
            "modified"
        } else {
            "unchanged"
        },
        format_duration(
            GLOBAL_TIME_RECORDER
                .total_elapsed()
                .change_context(Zerr::InternalError)?
        )
    );

    Ok(true)
}

fn render_inner(
    args: &crate::args::Args,
    render_args: &RenderCommand,
    raw_conf: config::RawConfig,
    lockfile: &mut self::lockfile::Lockfile,
    use_cli_initials: bool,
) -> Result<
    (
        Config,
        Vec<crate::render::template::Template>,
        Vec<crate::render::template::Template>,
    ),
    Zerr,
> {
    let conf = timeit!("Context value extraction (including scripting)", {
        config::process(raw_conf, Some(render_args), None, use_cli_initials)
    })?;

    let walker = timeit!("Filesystem walker creation", {
        self::walker::create(args, &render_args.root, &conf)
    })?;

    let templates = timeit!("Traversing filesystem & identifying templates", {
        self::walker::find_templates(&render_args.root, walker, "zetch")
    })?;

    let mut identical = Vec::new();
    let mut written = Vec::new();

    // Create the minijinja environment with the context.
    // A loader is set that can automatically load templates, this means it can load the main templates, and any other "includes" in user templates too.
    let env = timeit!("Creating rendering environment", {
        conf.engine.create_minijinja_env(&render_args.root, &conf)
    })?;

    timeit!("Rendering templates & syncing files", {
        for template in templates {
            debug!("Rendering template: {}", template.rel_path);
            let tmpl = env
                .get_template(&template.rel_path)
                .change_context(Zerr::InternalError)?;
            let compiled = match tmpl.render(context! {}) {
                Ok(compiled) => compiled,
                Err(e) => {
                    return Err(zerr!(
                        Zerr::RenderTemplateError,
                        "Failed to render template: '{}'",
                        e
                    ))
                }
            };
            let is_new = lockfile.add_template(&template, compiled)?;
            if is_new {
                written.push(template);
            } else {
                identical.push(template);
            }
        }
        Ok::<_, error_stack::Report<Zerr>>(())
    })?;

    Ok((conf, written, identical))
}

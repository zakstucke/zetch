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
    config::{self, Config},
    prelude::*,
};

pub fn render(args: &crate::args::Args, render_args: &RenderCommand) -> Result<bool, Zerr> {
    args_validate::args_validate(render_args)?;

    let mut lockfile = timeit!("Lockfile preparation", {
        self::lockfile::Lockfile::load(render_args.root.clone(), render_args.force)
    });

    let mut conf = config::load(
        args,
        Some(render_args),
        None,
        // If newly created, should use cli initials if specified:
        lockfile.newly_created,
    )?;

    // Need to run twice and rebuild config with real cli vars if initials used in the first conf build:
    let (written, identical) = if conf.cli_initials_used {
        warn!("Lockfile newly created/force updated and some cli vars have initials, will double render and use initials first time round.");
        // Conf from second as that has the real cli vars, template info from the first as the second will be inaccurate due to the first having run.
        let (first_written, first_identical) = render_inner(&conf, render_args, &mut lockfile)?;
        conf = config::load(args, Some(render_args), None, false)?;
        render_inner(&conf, render_args, &mut lockfile)?;
        (first_written, first_identical)
    } else {
        render_inner(&conf, render_args, &mut lockfile)?
    };

    // Run post-tasks:
    conf.tasks.run_post(&conf)?;

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
            matched_templates: {
                let mut all = vec![];
                for tmpl in written.iter() {
                    all.push(tmpl.rel_path.clone())
                }
                for tmpl in identical.iter() {
                    all.push(tmpl.rel_path.clone())
                }
                all
            },
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
    conf: &Config,
    render_args: &RenderCommand,
    lockfile: &mut self::lockfile::Lockfile,
) -> Result<
    (
        Vec<crate::render::template::Template>,
        Vec<crate::render::template::Template>,
    ),
    Zerr,
> {
    let walker = timeit!("Filesystem walker creation", {
        self::walker::create(&render_args.root, conf)
    })?;

    let templates = timeit!("Traversing filesystem & identifying templates", {
        self::walker::find_templates(&render_args.root, walker, conf.matchers.as_slice())
    })?;

    let mut identical = Vec::new();
    let mut written = Vec::new();

    // Create the minijinja environment with the context.
    // A loader is set that can automatically load templates, this means it can load the main templates, and any other "includes" in user templates too.
    let env = timeit!("Creating rendering environment", {
        conf.engine.create_minijinja_env(&render_args.root, conf)
    })?;

    timeit!("Rendering templates & syncing files", {
        for template in templates {
            debug!("Rendering template: {}", template.rel_path);
            let tmpl = match env.get_template(&template.rel_path) {
                Ok(tmpl) => Ok(tmpl),
                Err(e) => match e.kind() {
                    minijinja::ErrorKind::BadEscape => Err(e).change_context(Zerr::RenderTemplateError).attach_printable("Bad string escape in template. If windows filepaths being used in the template, make sure they've been escaped with an extra backslash. E.g. '.\\\\Desktop\\\\file.txt'"),
                    _ => Err(e).change_context(Zerr::InternalError),
                },
            }?;

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

    Ok((written, identical))
}

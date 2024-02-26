use bitbazaar::{
    timeit,
    timing::{format_duration, GLOBAL_TIME_RECORDER},
};
use colored::Colorize;
use minijinja::context;

mod args_validate;
mod debug;
mod lockfile;
mod mini_env;
mod template;
mod walker;
pub use lockfile::hash_contents;
pub use walker::get_template_matcher_rewrite_mapping;

use crate::{args::RenderCommand, prelude::*, render::mini_env::new_mini_env, state::State};

pub fn render(args: &crate::args::Args, render_args: &RenderCommand) -> Result<bool, Zerr> {
    args_validate::args_validate(render_args)?;

    let mut lockfile = timeit!("Lockfile preparation", {
        self::lockfile::Lockfile::load(render_args.root.clone(), render_args.force)
    });

    // TODO double prints what's that about

    let mut state = State::new(args)?;
    state.load_all_vars()?;
    debug!("State: {:#?}", state);

    let (written, identical) = render_inner(&state, render_args, &mut lockfile)?;

    // Run post-tasks only if not light/superlight:
    if !state.light {
        state.conf.tasks.run_post(&state)?;
    }

    timeit!("Syncing lockfile", { lockfile.sync() })?;

    // Write only when hidden cli flag --debug is set, to allow testing internals from python without having to setup custom interfaces:
    if render_args.debug {
        let debug = debug::Debug {
            conf: state.conf.clone(),
            ctx: state.ctx.clone(),
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

    let num_tasks = if state.light {
        0
    } else {
        state.conf.tasks.pre.len() + state.conf.tasks.post.len()
    };
    println!(
        "{} {} template{} written, {} identical.{} Lockfile {}. {} elapsed.",
        "zetch:".bold(),
        written.len(),
        if written.len() == 1 { "" } else { "s" },
        identical.len(),
        if num_tasks > 0 {
            format!("{} tasks run.", num_tasks).to_string()
        } else {
            "".to_string()
        },
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
    state: &State,
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
        self::walker::create(&render_args.root, state)
    })?;

    let templates = timeit!("Traversing filesystem & identifying templates", {
        self::walker::find_templates(&render_args.root, walker, state.conf.matchers.as_slice())
    })?;

    let mut identical = Vec::new();
    let mut written = Vec::new();

    // Create the minijinja environment with the context.
    // A loader is set that can automatically load templates, this means it can load the main templates, and any other "includes" in user templates too.
    let env = timeit!("Creating rendering environment", {
        new_mini_env(&render_args.root, state)
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
                    let mut out_e = zerr!(Zerr::RenderTemplateError, "Failed to render template.")
                        .attach_printable(format!("{}", e));

                    // Rendering failed, important here to give a really nice error as common user error.
                    // So print the lines around the error if possible:
                    if let Some(err_line_no) = e.line() {
                        let source_code = std::fs::read_to_string(&template.path)
                            .change_context(Zerr::InternalError)?;
                        let lines = source_code.lines().collect::<Vec<_>>();
                        let start_line_no = if err_line_no > 3 { err_line_no - 3 } else { 1 };
                        let end_line_no = (err_line_no + 3).min(lines.len());
                        let mut s = String::new();
                        for line_no in start_line_no..(end_line_no + 1) {
                            // Handle keeping aligned, e.g. line numbers start in single digits but go into double digits:
                            let extra_indent = " "
                                .repeat(end_line_no.to_string().len() - line_no.to_string().len());
                            let line = lines[line_no - 1];
                            if line_no == err_line_no {
                                // If possible, identify the portion of the line causing the error, making it bright red and underlined:
                                // This will only be known in some cases, e.g. mj exposes it for {{ REE + 1 }} when REE is undefined but not {{ REE }}
                                let fmtted_line = if let Some(source_range) = e.range() {
                                    // The range is of the entire template, not the line, so need to normalise it for the line:
                                    let mut offset = 0;
                                    for l in lines.iter().take(line_no - 1) {
                                        offset += l.len() + 1;
                                    }
                                    let line_range =
                                        source_range.start - offset..source_range.end - offset;
                                    // If line range finishes greater than the length of the line, raise internal error as it's something wrong with this block:
                                    if line_range.end > line.len() {
                                        return Err(zerr!(
                                            Zerr::InternalError,
                                            "Line range end is greater than line length."
                                        ));
                                    }
                                    format!(
                                        "{}{}{}",
                                        &line[..line_range.start],
                                        &line[line_range.clone()].underline().bright_red(),
                                        &line[line_range.end..]
                                    )
                                } else {
                                    line.to_string()
                                };
                                s.push_str(&format!(
                                    "{}",
                                    format!(
                                        "{}{}| {} <-- ERR\n",
                                        extra_indent, line_no, fmtted_line
                                    )
                                    .red()
                                    .bold()
                                ));
                            } else {
                                s.push_str(&format!("{}{}| {}\n", extra_indent, line_no, line));
                            }
                        }
                        out_e = out_e.attach_printable(s);
                    }

                    return Err(out_e);
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

use std::fs::create_dir_all;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{cmp, env, io, str};

use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Command, CommandFactory};
use once_cell::sync::Lazy;
use regex::Regex;
use tracing::error;

use crate::branding::BrandingCompileEnvVars;
use crate::{docs, OckamCommand};

#[derive(Clone, Debug, Args)]
#[command(
about = docs::about("Generate markdown files for all existing Ockam commands"),
hide = docs::hide())]
pub struct MarkdownCommand {
    #[arg(short, long, value_parser(NonEmptyStringValueParser::new()))]
    #[arg(help = docs::about("\
    Absolute path to the output directory where the generated markdown files will be stored. \
    Defaults to \"./ockam_markdown_pages\" in the current working directory."))]
    dir: Option<String>,
}

impl MarkdownCommand {
    pub fn run(self) -> miette::Result<()> {
        let mark_dir = match get_markdown_page_directory(&self.dir) {
            Ok(path) => path,
            Err(error) => panic!("Error getting markdown page directory: {error:?}"),
        };
        env::set_var("OCKAM_HELP_RENDER_MARKDOWN", "1");
        let clap_command = <OckamCommand as CommandFactory>::command();

        let mut summary: String = String::from("# Summary\n\n");
        generate_markdown_pages(
            mark_dir.as_path(),
            &clap_command,
            None,
            Vec::new(),
            &mut summary,
        );

        std::fs::write(mark_dir.join("SUMMARY.md"), summary).expect("Error creating SUMMARY.md.");
        Ok(())
    }

    pub fn name(&self) -> String {
        "generate markdown".to_string()
    }
}

fn get_markdown_page_directory(cmd_mark_dir: &Option<String>) -> io::Result<PathBuf> {
    let mark_dir = match cmd_mark_dir {
        Some(dir) => {
            let mut user_specified_dir = PathBuf::new();
            user_specified_dir.push(dir);
            user_specified_dir
        }
        None => {
            let mut mark_dir = env::current_dir()?;
            mark_dir.push(format!(
                "{}_markdown_pages",
                BrandingCompileEnvVars::bin_name()
            ));
            println!("Markdown pages stored at: {}", mark_dir.display());
            mark_dir
        }
    };

    create_dir_all(mark_dir.clone())?;
    Ok(mark_dir)
}

fn generate_markdown_pages(
    mark_dir: &Path,
    cmd: &Command,
    name: Option<&str>,
    parent_cmd: Vec<String>,
    summary: &mut String,
) {
    let cmd_name = match name {
        None => cmd.get_name(),
        Some(name) => name,
    };

    let indent = cmp::max(parent_cmd.len(), 1) - 1;
    let summary_line = format!(
        "{} - [{}](./{}.md)\n",
        "    ".repeat(indent),
        cmd.get_name(),
        cmd_name
    );
    summary.push_str(summary_line.as_str());

    // generate markdown page for command
    match generate_markdown_page(mark_dir, cmd_name, cmd, &parent_cmd) {
        Ok(()) => (),
        Err(error) => error!(
            "Error generating markdown page for command \"{}\": {:?}",
            cmd_name, error
        ),
    }

    let parent_cmd = {
        let mut parent_cmd = parent_cmd;
        parent_cmd.push(cmd.get_name().to_owned());
        parent_cmd
    };

    // generate markdown page for sub commands
    for s_cmd in cmd.get_subcommands() {
        let sub_cmd_name = [cmd_name, "-", s_cmd.get_name()].concat();

        // skip in case subcommand is hidden within help
        if s_cmd.is_hide_set() {
            continue;
        }

        // recurse to cover all subcommand levels
        generate_markdown_pages(
            mark_dir,
            s_cmd,
            Some(&sub_cmd_name),
            parent_cmd.clone(),
            summary,
        );
    }
}

fn generate_markdown_page(
    dir: &Path,
    name: &str,
    cmd: &Command,
    parent_cmd: &[String],
) -> io::Result<()> {
    let mut buffer = Vec::<u8>::new();
    let buffer = &mut buffer;

    let mut p_cmd = get_parent_commands(parent_cmd, " ");

    // Title
    write!(
        buffer,
        "## {} {} ",
        p_cmd.replace(&format!("{} ", BrandingCompileEnvVars::bin_name()), ""),
        cmd.get_name()
    )?;

    // Separator after title
    writeln!(buffer, "\n---\n")?;

    // Before help: used to print the `Preview` tag
    if let Some(s) = cmd.get_before_help().map(|s| s.to_string()) {
        if !s.is_empty() {
            writeln!(buffer, "{}", s)?;
        }
    }

    // Usage (e.g. "ockam space create [OPTIONS] [NAME] [-- <ADMINS>...]")
    let mut usage = cmd.clone().render_usage().to_string();
    // remove `Usage:` from the string
    usage = usage.replace("Usage: ", "");
    // append parent commands in beginning of the usage
    writeln!(buffer, "`{} {} `\n", p_cmd, usage)?;

    // Long about; fallback to short about
    if let Some(s) = cmd.get_long_about().map(|s| s.to_string()) {
        if !s.is_empty() {
            writeln!(buffer, "{}", process_txt_to_md(s))?;
        }
    } else if let Some(s) = cmd.get_about().map(|s| s.to_string()) {
        if !s.is_empty() {
            writeln!(buffer, "{}", process_txt_to_md(s))?;
        }
    }

    // Arguments and options if the command has no subcommands
    if cmd.get_subcommands().next().is_none() {
        // Arguments
        if cmd.get_positionals().next().is_some() {
            writeln!(buffer, "### Arguments\n")?;

            for pos_arg in cmd.get_positionals() {
                generate_arg_markdown(buffer, pos_arg)?;
            }

            writeln!(buffer)?;
        }

        // Options
        let non_pos: Vec<_> = cmd
            .get_arguments()
            .filter(|arg| !arg.is_positional())
            .collect();

        if !non_pos.is_empty() {
            writeln!(buffer, "### Options\n")?;

            for arg in non_pos {
                generate_arg_markdown(buffer, arg)?;
            }

            writeln!(buffer)?;
        }
    } else {
        // Subcommands list
        writeln!(buffer, "### Subcommands\n")?;

        for s_cmd in cmd.get_subcommands() {
            if s_cmd.is_hide_set() {
                continue;
            }

            p_cmd = get_parent_commands(parent_cmd, "-");
            if !p_cmd.is_empty() {
                p_cmd.push('-');
            }
            writeln!(
                buffer,
                "* [{}]({}{}-{}.md)",
                s_cmd.get_name(),
                p_cmd,
                cmd.get_name(),
                s_cmd.get_name()
            )?;
        }
        writeln!(buffer)?;
    }

    // After help: print either the long or the short version
    if let Some(s) = cmd.get_after_help() {
        if !s.to_string().is_empty() {
            writeln!(buffer, "{}\n", s)?;
        }
    } else if let Some(s) = cmd.get_after_long_help() {
        if !s.to_string().is_empty() {
            writeln!(buffer, "{}", process_txt_to_md(s.to_string()))?;
        }
    }

    // make a .md file and add the buffer to it
    let mut name = name.to_owned();
    name.push_str(".md");
    std::fs::write(dir.join(name), buffer)?;
    Ok(())
}

fn get_parent_commands(parent_cmd: &[String], separator: &str) -> String {
    if parent_cmd.is_empty() {
        String::new()
    } else {
        parent_cmd.join(separator)
    }
}

fn generate_arg_markdown(buffer: &mut Vec<u8>, arg: &clap::Arg) -> io::Result<()> {
    write!(buffer, "* ")?;

    let value_name: String = match arg.get_value_names() {
        Some([name, ..]) => name.as_str().to_owned(),
        Some([]) => unreachable!(),
        None => arg.get_id().to_string().to_ascii_uppercase(),
    };
    let (formatted_value_name, optional) = match arg.is_required_set() {
        true => (format!("<{value_name}>"), ""),
        false => (format!("[{value_name}]"), " (optional)"),
    };

    match (arg.get_short(), arg.get_long()) {
        (Some(short), Some(long)) => {
            if arg.get_action().takes_values() {
                write!(buffer, "`-{short}`, `--{long} {formatted_value_name}`")?
            } else {
                write!(buffer, "`-{short}`, `--{long}`")?
            }
        }
        (Some(short), None) => {
            if arg.get_action().takes_values() {
                write!(buffer, "`-{short} {formatted_value_name}`")?
            } else {
                write!(buffer, "`-{short}`")?
            }
        }
        (None, Some(long)) => {
            if arg.get_action().takes_values() {
                write!(buffer, "`--{} {formatted_value_name}`", long)?
            } else {
                write!(buffer, "`--{}`", long)?
            }
        }
        (None, None) => {
            write!(buffer, "`{formatted_value_name}`")?;
        }
    }
    write!(buffer, "{optional}")?;

    if let Some(help) = arg.get_help() {
        writeln!(buffer, "<br/>")?;
        writeln!(buffer, "{help}\n")?;
    } else {
        writeln!(buffer)?;
    }

    Ok(())
}

static SUBHEADER3: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([\w :,.]+)\n(-{6})").expect("Invalid regex for SUBHEADER3"));

fn process_txt_to_md(contents: String) -> String {
    // Converts the following:
    //   <TEXT>
    //   ------
    // To: ### <TEXT>
    let res = SUBHEADER3.replace_all(&contents, "### $1");
    res.to_string()
}

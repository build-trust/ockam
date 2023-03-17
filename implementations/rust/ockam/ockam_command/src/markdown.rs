use crate::docs;
use crate::OckamCommand;
use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Command, CommandFactory};
use std::fs::create_dir_all;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, io, str};
use tracing::error;

const LONG_HELP: &str = "\
markdown pages output directory. Absolute path required. Will be created in case not existing. \
Fallback: \"ockam_markdown_pages/\" in the current working directory.";

/// Generate Ockam markdown pages
#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide())]
pub struct MarkdownCommand {
    #[arg(
        short,
        long,
        help = "markdown output directory path",
        long_help = LONG_HELP,
        value_parser(NonEmptyStringValueParser::new())
    )]
    dir: Option<String>,
}

impl MarkdownCommand {
    pub fn run(self) {
        let mark_dir = match get_markdown_page_directory(&self.dir) {
            Ok(path) => path,
            Err(error) => panic!("Error getting markdown page directory: {error:?}"),
        };
        env::set_var("MARKDOWN_RENDER", "1");
        let clap_command = <OckamCommand as CommandFactory>::command();
        generate_markdown_pages(mark_dir.as_path(), &clap_command, None, Vec::new());
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
            mark_dir.push("ockam_markdown_pages/");
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
) {
    let cmd_name = match name {
        None => cmd.get_name(),
        Some(name) => name,
    };

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
        // skip in case subcommand is hidden within help
        if s_cmd.is_hide_set() {
            continue;
        }

        // recurse to cover all subcommand levels
        let sub_cmd_name = [cmd_name, "-", s_cmd.get_name()].concat();
        generate_markdown_pages(mark_dir, s_cmd, Some(&sub_cmd_name), parent_cmd.clone());
    }
}

fn generate_markdown_page(
    dir: &Path,
    name: &str,
    cmd: &Command,
    parent_cmd: &Vec<String>,
) -> io::Result<()> {
    let mut buffer = Vec::<u8>::new();
    let buffer = &mut buffer;

    let mut p_cmd = get_parent_commands(parent_cmd, " ");
    if !p_cmd.is_empty() {
        p_cmd.push(' ');
    }

    // Title
    writeln!(
        buffer,
        "## {} {}\n",
        p_cmd.replace("ockam ", ""),
        cmd.get_name()
    )?;
    writeln!(buffer, "---")?;

    // command usage template
    let mut usage = cmd.clone().render_usage().to_string();
    // remove `usage:` from the string
    usage = usage.replace("Usage: ", "");
    // append parent commands in beginning of the usage
    writeln!(buffer, "`{}{}`\n", p_cmd, usage)?;

    // Before help
    if let Some(s) = cmd.get_before_long_help() {
        writeln!(buffer, "{}\n", s)?;
    } else if let Some(s) = cmd.get_before_help() {
        writeln!(buffer, "{}\n", s)?;
    }

    // About: print the short version first, then the long version.
    if let Some(about) = cmd.get_about() {
        writeln!(buffer, "{}.\n", about.to_string().trim_end_matches('.'))?;
    }
    if let Some(about) = cmd.get_long_about() {
        writeln!(buffer, "{}\n", about)?;
    }

    // Subcommands list
    if cmd.get_subcommands().next().is_some() {
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

    // After help
    if let Some(s) = cmd.get_after_long_help() {
        writeln!(buffer, "{}\n", s)?;
    } else if let Some(s) = cmd.get_after_help() {
        writeln!(buffer, "{}\n", s)?;
    }

    // make a .md file and add the buffer to it
    let mut name = name.to_owned();
    name.push_str(".md");
    std::fs::write(dir.join(name), &buffer)?;
    Ok(())
}

fn get_parent_commands(parent_cmd: &Vec<String>, separator: &str) -> String {
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

    match (arg.get_short(), arg.get_long()) {
        (Some(short), Some(long)) => {
            if arg.get_action().takes_values() {
                write!(buffer, "`-{short}`, `--{long} <{value_name}>`")?
            } else {
                write!(buffer, "`-{short}`, `--{long}`")?
            }
        }
        (Some(short), None) => {
            if arg.get_action().takes_values() {
                write!(buffer, "`-{short} <{value_name}>`")?
            } else {
                write!(buffer, "`-{short}`")?
            }
        }
        (None, Some(long)) => {
            if arg.get_action().takes_values() {
                write!(buffer, "`--{} <{value_name}>`", long)?
            } else {
                write!(buffer, "`--{}`", long)?
            }
        }
        (None, None) => {
            write!(buffer, "`<{value_name}>`",)?;
        }
    }

    if let Some(help) = arg.get_help() {
        writeln!(buffer, "<br/>")?;
        writeln!(buffer, "{help}\n")?;
    } else {
        writeln!(buffer)?;
    }

    Ok(())
}

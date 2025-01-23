use std::fs::{create_dir_all, File};
use std::io::{Error, Write};
use std::path::{Path, PathBuf};
use std::{env, str};

use clap::builder::NonEmptyStringValueParser;
use clap::{ArgAction, Args, Command, CommandFactory};
use clap_mangen::Man;
use flate2::{Compression, GzBuilder};
use miette::IntoDiagnostic;
use tracing::error;

use ockam_core::env::get_env_with_default;

use crate::environment::compile_time_vars::BIN_NAME;
use crate::{docs, OckamCommand};

#[derive(Clone, Debug, Args)]
#[command(
about = docs::about("Generate man pages for all existing Ockam commands"),
hide = docs::hide()
)]
pub struct ManpagesCommand {
    #[arg(short, long, value_parser(NonEmptyStringValueParser::new()))]
    #[arg(help = docs::about("\
    Absolute path to the output directory where the generated man pages will be stored. \
    Defaults to \"~/local/.share/man/man1/\"; fallback to \"./ockam_man_pages\"."))]
    dir: Option<String>,

    #[arg(
        short,
        long,
        default_value = "false",
        action = ArgAction::SetTrue,
        help = "disable gzip compression for man page output",
    )]
    no_compression: bool,
}

impl ManpagesCommand {
    pub fn run(self) -> miette::Result<()> {
        let man_dir = match get_man_page_directory(&self.dir) {
            Ok(path) => path,
            Err(error) => panic!("Error getting man page directory: {error:?}"),
        };
        let clap_command = <OckamCommand as CommandFactory>::command();
        generate_man_pages(man_dir.as_path(), &clap_command, None, self.no_compression);
        Ok(())
    }

    pub fn name(&self) -> String {
        "manpages".to_string()
    }
}

fn get_man_page_directory(cmd_man_dir: &Option<String>) -> crate::Result<PathBuf> {
    let man_dir = match cmd_man_dir {
        Some(dir) => {
            let mut user_specified_dir = PathBuf::new();
            user_specified_dir.push(dir);
            user_specified_dir
        }
        None => match get_env_with_default("HOME", None::<String>)
            .into_diagnostic()?
            .map(PathBuf::from)
        {
            Some(mut home_dir) => {
                home_dir.push(".local/share/man/man1");
                home_dir
            }
            None => {
                let mut man_dir = env::current_dir().into_diagnostic()?;
                man_dir.push(format!("{BIN_NAME}_man_pages"));
                println!("Man pages stored at: {}", man_dir.display());
                man_dir
            }
        },
    };
    create_dir_all(&man_dir).into_diagnostic()?;
    Ok(man_dir)
}

fn generate_man_pages(man_dir: &Path, cmd: &Command, name: Option<&str>, no_compression: bool) {
    let cmd_name = match name {
        None => cmd.get_name(),
        Some(name) => name,
    };

    // generate man page for command
    match generate_man_page(man_dir, cmd_name, cmd, no_compression) {
        Ok(()) => (),
        Err(error) => error!(
            "Error generating man page for command \"{}\": {:?}",
            cmd_name, error
        ),
    }

    // generate man page for sub commands
    for s_cmd in cmd.get_subcommands() {
        // skip in case subcommand is hidden within help
        if s_cmd.is_hide_set() {
            continue;
        }

        // recurse to cover all subcommand levels
        let sub_cmd_name = [cmd_name, "-", s_cmd.get_name()].concat();
        generate_man_pages(man_dir, s_cmd, Some(&sub_cmd_name), no_compression)
    }
}

fn generate_man_page(
    dir: &Path,
    name: &str,
    cmd: &Command,
    no_compression: bool,
) -> Result<(), Error> {
    let man = Man::new(cmd.clone());
    let mut render: Vec<u8> = Default::default();
    man.render(&mut render)?;
    let render_cleaned = remove_ascii_controls(render);

    let mut name: String = name.to_owned();
    name.push_str(".1");

    if no_compression {
        std::fs::write(dir.join(name), render_cleaned)?;
    } else {
        let mut name_gz = name.clone();
        name_gz.push_str(".gz");
        let output_file = File::create(dir.join(name_gz))?;

        let mut gz = GzBuilder::new()
            .filename(name)
            .write(output_file, Compression::default());

        gz.write_all(&render_cleaned)?;
        gz.finish()?;
    }
    Ok(())
}

fn remove_ascii_controls(input: Vec<u8>) -> Vec<u8> {
    let input_as_str = match str::from_utf8(&input) {
        Ok(input) => input,
        Err(e) => panic!("Input contains non UTF-8 sequence: {e}"),
    };

    let mut result: Vec<u8> = Default::default();

    let mut control_sequence = false;
    let control_terminate: char = 'm';
    for ch in input_as_str.chars() {
        if ch.is_ascii_control() && !ch.is_ascii_whitespace() {
            control_sequence = true;
        } else if control_sequence && ch == control_terminate {
            control_sequence = false;
            continue;
        }
        if !control_sequence {
            result.push(ch as u8);
        }
    }

    result
}

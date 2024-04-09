use crate::GlobalArgs;
use console::Term;
use ockam_api::terminal::{Terminal, TerminalStream};

impl From<&GlobalArgs> for Terminal<TerminalStream<Term>> {
    fn from(global_args: &GlobalArgs) -> Self {
        Terminal::new(
            global_args.quiet,
            global_args.no_color,
            global_args.no_input,
            global_args.output_format.clone(),
        )
    }
}

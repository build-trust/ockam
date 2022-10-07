use ockam_abac::{eval, parse, Env, Expr};
use rustyline::error::ReadlineError;
use rustyline::highlight::MatchingBracketHighlighter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::{Config, EditMode, Editor, Result};
use rustyline_derive::{Completer, Helper, Highlighter, Hinter, Validator};

const HELP: &str = r#"Available commands:
  :def <id> <expression>  -- Add an expression to the environment.
  :env                    -- Show all current environment entries.
  :clear                  -- Remove all bindings from the environment.
  :help | :h | :?         -- Show this help message."#;

#[derive(Completer, Helper, Highlighter, Hinter, Validator)]
struct ReplHelper {
    #[rustyline(Highlighter)]
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
}

fn main() -> Result<()> {
    let c = Config::builder()
        .edit_mode(EditMode::Vi)
        .auto_add_history(true)
        .history_ignore_space(true)
        .build();

    let mut env = Env::new();
    let mut repl = Editor::<ReplHelper>::with_config(c)?;
    repl.set_helper(Some(ReplHelper {
        highlighter: MatchingBracketHighlighter::new(),
        validator: MatchingBracketValidator::new(),
    }));

    loop {
        let readline = repl.readline("❱ ");
        match readline {
            Ok(line) => {
                if line.starts_with(':') {
                    on_command(&line, &mut env)
                } else {
                    match parse(&line) {
                        Ok(None) => continue,
                        Ok(Some(e)) => match eval(&e, &env) {
                            Ok(x) => println!("{x}"),
                            Err(e) => eprintln!("error: {e}"),
                        },
                        Err(e) => eprintln!("error: {e}"),
                    }
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("error: {e}");
                break;
            }
        }
    }

    Ok(())
}

fn on_command(line: &str, env: &mut Env) {
    let i = line.find(|c: char| c.is_whitespace()).unwrap_or(line.len());
    match line.split_at(i) {
        (":def", rest) => match parse(rest) {
            Ok(Some(Expr::List(xs))) => {
                if let [Expr::Ident(name), e] = &xs[..] {
                    match eval(e, env) {
                        Ok(x) => {
                            env.put(name, x);
                        }
                        Err(e) => eprintln!("error: {e}"),
                    }
                } else {
                    eprintln!("invalid :def command")
                }
            }
            Ok(_) => eprintln!("invalid :def command"),
            Err(e) => eprintln!("error: {e}"),
        },
        (":env", _) => {
            for (id, expr) in env.entries() {
                println!("{id} {expr}")
            }
        }
        (":clear", _) => env.clear(),
        (":help" | ":h" | ":?", _) => println!("{HELP}"),
        (cmd, _) => eprintln!("unknown command {cmd}"),
    }
}

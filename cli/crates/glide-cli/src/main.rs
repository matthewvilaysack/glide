use std::process::ExitCode as ProcessExitCode;

use clap::Parser;
use glide_common::errors::ExitCode;
use tracing_subscriber::EnvFilter;

mod cli;
mod commands;
mod output;

use cli::{Cli, Command};

/// Verbs that live under a parent command, and the parent they belong to.
///
/// People type the verb they mean, not the path to it. `glide log` is what a hand
/// reaches for even though the command is `glide focus log`, and clap's nearest
/// match does not look inside subcommands, so it answered `glide log` with "a
/// similar subcommand exists: 'init'". A wrong suggestion is worse than none,
/// because the next thing the reader does is run it.
const NESTED_VERBS: &[(&str, &str)] = &[
    ("log", "focus"),
    ("set", "focus"),
    ("done", "focus"),
    ("capture", "focus"),
    ("list", "focus"),
    ("clear", "focus"),
    ("add", "focus"),
    ("start", "sprint"),
    ("pull", "sprint"),
    ("end", "sprint"),
];

/// What someone meant by a word that is not a top-level command.
///
/// `add` is the special case worth spelling out: there is no `focus add`, the verb
/// is `capture`, and being told to run a second command that also does not exist
/// is the same dead end twice.
fn nested_hint(word: &str) -> Option<String> {
    if word == "add" {
        return Some(
            "`add` is not a verb here. To put something in today's list, use `glide focus capture <text>`."
                .to_string(),
        );
    }
    NESTED_VERBS
        .iter()
        .find(|(v, _)| *v == word)
        .map(|(v, parent)| format!("did you mean `glide {parent} {v}`?"))
}

fn main() -> ProcessExitCode {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            // Only improve the "no such command" case; every other clap error is
            // already precise and clap prints it better than this could.
            if e.kind() == clap::error::ErrorKind::InvalidSubcommand {
                let args: Vec<String> = std::env::args().skip(1).take(2).collect();
                // `glide add` and `glide focus add` are the same mistake in two
                // positions, so both get the same answer.
                let bad = match args.as_slice() {
                    [parent, verb] if NESTED_VERBS.iter().any(|(_, p)| p == parent) => verb.clone(),
                    [word, ..] => word.clone(),
                    [] => String::new(),
                };
                if let Some(hint) = nested_hint(&bad) {
                    println!("error: `{bad}` is not a glide verb");
                    println!("help: {hint}");
                    return ProcessExitCode::from(ExitCode::Config as u8);
                }
            }
            e.print().ok();
            return ProcessExitCode::from(match e.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    ExitCode::Ok
                }
                _ => ExitCode::Config,
            } as u8);
        }
    };
    init_tracing(cli.debug);

    let result = dispatch(cli);
    match result {
        Ok(()) => ProcessExitCode::from(ExitCode::Ok as u8),
        Err(e) => {
            eprintln!("error: {e:#}");
            // Map back to a glide error if we can; otherwise general.
            let code = e
                .downcast_ref::<glide_common::GlideError>()
                .map(ExitCode::from_error)
                .unwrap_or(ExitCode::General);
            ProcessExitCode::from(code as u8)
        }
    }
}

fn dispatch(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Some(Command::Console) => console(&cli.globals),
        Some(Command::Init(args)) => commands::init::run(&cli.globals, args),
        Some(Command::Index(args)) => commands::index::run(&cli.globals, args),
        Some(Command::WhoOwns(args)) => commands::who_owns::run(&cli.globals, args),
        Some(Command::Plan(args)) => commands::plan::run(&cli.globals, args),
        Some(Command::Request(args)) => commands::request::run(&cli.globals, args),
        Some(Command::Friction(args)) => commands::friction::run(&cli.globals, args),
        Some(Command::Config(args)) => commands::config::run(&cli.globals, args),
        Some(Command::Doctor) => commands::doctor::run(&cli.globals),
        Some(Command::Models) => commands::models::run(&cli.globals),
        Some(Command::Tools(args)) => commands::tools::run(&cli.globals, args),
        Some(Command::Focus(args)) => commands::focus::run(&cli.globals, args),
        Some(Command::Sprint(args)) => commands::sprint::run(&cli.globals, args),
        Some(Command::Decide(args)) => commands::decide::run(&cli.globals, args),
        Some(Command::Decisions) => commands::decide::list(&cli.globals),
        Some(Command::Show) => commands::focus::show_list(&cli.globals),
        Some(Command::Today) => commands::focus::run(
            &cli.globals,
            cli::FocusCmd {
                sub: Some(cli::FocusSub::Today),
            },
        ),
        Some(Command::Build(args)) => commands::index::run(
            &cli.globals,
            cli::IndexCmd {
                sub: cli::IndexSub::Build(args),
            },
        ),
        Some(Command::Mcp(args)) => commands::mcp::run(&cli.globals, args),
        Some(Command::Prime(args)) => commands::prime::run(&cli.globals, args),
        Some(Command::Setup(args)) => commands::setup::run(&cli.globals, args),
        Some(Command::Onboard) => {
            println!("{}", commands::setup::onboard_text());
            Ok(())
        }
        Some(Command::Completion(args)) => commands::completion::run(args),
        None => {
            if let Some(prompt) = cli.prompt.clone() {
                commands::one_shot::run(&cli.globals, &prompt)
            } else if is_terminal::IsTerminal::is_terminal(&std::io::stdout()) {
                // Bare `glide` on a terminal opens the console. Piped or
                // redirected, it still prints help, so scripts don't change.
                console(&cli.globals)
            } else {
                use clap::CommandFactory;
                Cli::command().print_help()?;
                println!();
                Ok(())
            }
        }
    }
}

#[cfg(feature = "console")]
fn console(globals: &cli::GlobalArgs) -> anyhow::Result<()> {
    commands::console::run(globals)
}

#[cfg(not(feature = "console"))]
fn console(_globals: &cli::GlobalArgs) -> anyhow::Result<()> {
    anyhow::bail!("this build was compiled without the `console` feature")
}

fn init_tracing(debug: bool) {
    let filter = if debug {
        EnvFilter::new("glide=debug,info")
    } else {
        EnvFilter::try_from_env("GLIDE_LOG").unwrap_or_else(|_| EnvFilter::new("warn"))
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_verb_that_lives_under_focus_points_at_focus() {
        assert_eq!(
            nested_hint("log").unwrap(),
            "did you mean `glide focus log`?"
        );
        assert_eq!(
            nested_hint("capture").unwrap(),
            "did you mean `glide focus capture`?"
        );
    }

    #[test]
    fn a_verb_that_lives_under_sprint_points_at_sprint() {
        assert_eq!(
            nested_hint("pull").unwrap(),
            "did you mean `glide sprint pull`?"
        );
    }

    #[test]
    fn add_is_told_the_real_verb_rather_than_a_command_that_also_does_not_exist() {
        let h = nested_hint("add").unwrap();
        assert!(h.contains("glide focus capture"), "{h}");
        assert!(!h.contains("focus add"), "{h}");
    }

    #[test]
    fn a_word_that_is_not_a_verb_gets_no_invented_suggestion() {
        assert!(nested_hint("spirnt").is_none());
        assert!(nested_hint("xyzzy").is_none());
    }

    #[test]
    fn every_nested_verb_names_a_real_parent() {
        for (_, parent) in NESTED_VERBS {
            assert!(
                matches!(*parent, "focus" | "sprint"),
                "{parent} is not a command"
            );
        }
    }
}

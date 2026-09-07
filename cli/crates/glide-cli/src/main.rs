use std::process::ExitCode as ProcessExitCode;

use clap::Parser;
use glide_common::errors::ExitCode;
use tracing_subscriber::EnvFilter;

mod cli;
mod commands;
mod output;

use cli::{Cli, Command};

fn main() -> ProcessExitCode {
    let cli = Cli::parse();
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

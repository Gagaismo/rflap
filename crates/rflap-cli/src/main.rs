//! `rflap`: the command-line tool of Rflap, a thin layer over `rflap-api`.
//!
//! Every command builds an API request, calls [`rflap_api`] and prints the answer as clear
//! text, or as the raw JSON answer with `--json`. No algorithm lives here.

mod api;
mod caps;
mod cli;
mod design;
mod error;
mod grammar;
mod gui;
mod input;
mod machine;
mod pumping;
mod text;

use std::io::Write;

use clap::Parser;

use cli::{Cli, Command, PumpingCommand};
use error::Result;
use text::{Reply, exit};

fn dispatch(cli: &Cli) -> Result<Reply> {
    match &cli.command {
        Command::Call { request } => machine::call(request.as_deref()),
        Command::Info(doc) => machine::info(doc),
        Command::Run {
            inline,
            args,
            inputs_file,
            trace,
            options,
        } => machine::run(inline, args, inputs_file.as_deref(), *trace, options),
        Command::Step {
            inline,
            args,
            steps,
            options,
        } => machine::step(inline, args, *steps, options),
        Command::Convert {
            doc,
            to,
            output,
            steps,
        } => machine::convert(doc, to, output.as_deref(), *steps),
        Command::Equiv { a, b } => machine::equiv(a, b),
        Command::Combine { a, b, output } => machine::combine(a, b, output),
        Command::Render {
            doc,
            format,
            output,
            dark,
            highlight,
            highlight_transitions,
            depth,
        } => machine::render(
            doc,
            *format,
            output.as_deref(),
            *dark,
            highlight,
            highlight_transitions,
            *depth,
        ),
        Command::Layout {
            doc,
            algorithm,
            output,
        } => machine::layout(doc, algorithm, output),
        Command::GrammarType(args) => grammar::grammar_type(args),
        Command::Parse {
            inline,
            args,
            parser,
            tree,
            budget,
        } => grammar::parse(inline, args, *parser, *tree, *budget),
        Command::Tables(args) => grammar::tables(args),
        Command::Design {
            constraint,
            regex,
            grammar,
            template,
            param,
            alphabet,
            accept,
            reject,
            output,
            svg,
        } => design::design(&design::DesignArgs {
            constraint: constraint.as_deref(),
            regex: regex.as_deref(),
            grammar: grammar.as_deref(),
            template: template.as_deref(),
            params: param,
            alphabet,
            accept,
            reject,
            output: output.as_deref(),
            svg: svg.as_deref(),
        }),
        Command::Explain(doc) => machine::explain(doc),
        Command::Lsystem {
            file,
            depth,
            output,
        } => machine::lsystem(file, *depth, output.as_deref()),
        Command::Pumping { command } => match command {
            PumpingCommand::List => pumping::list(),
            PumpingCommand::Play {
                kind,
                language,
                mode,
                seed,
                moves,
            } => pumping::play(kind, language, mode, *seed, moves, cli.json),
        },
        Command::Capabilities => caps::capabilities(),
        Command::Gui {
            file,
            port,
            no_open,
        } => {
            gui::run(file.as_deref(), *port, *no_open)?;
            Ok(Reply {
                text: String::new(),
                json: String::new(),
                code: exit::OK,
            })
        }
    }
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let code = if e.use_stderr() {
                exit::USAGE
            } else {
                exit::OK
            };
            e.print().ok();
            std::process::exit(code);
        }
    };
    let raw_json = cli.json || matches!(cli.command, Command::Call { .. });
    let mut stdout = std::io::stdout().lock();
    let code = match dispatch(&cli) {
        Ok(reply) => {
            let shown = if raw_json { &reply.json } else { &reply.text };
            if !shown.is_empty() {
                let newline = if raw_json && !shown.ends_with('\n') {
                    "\n"
                } else {
                    ""
                };
                write!(stdout, "{shown}{newline}").ok();
            }
            reply.code
        }
        Err(e) => {
            if raw_json {
                writeln!(stdout, "{}", e.json()).ok();
            }
            eprintln!("error: {e}");
            exit::USAGE
        }
    };
    stdout.flush().ok();
    std::process::exit(code);
}

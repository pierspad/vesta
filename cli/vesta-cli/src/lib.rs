use std::ffi::OsString;
use std::process::ExitCode;

use clap::{Command, error::ErrorKind};

fn command() -> Command {
    Command::new("vesta")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Vesta headless subtitle and flashcard workflows")
        .long_about("Run headless Vesta operations. Each operation forwards its options to the same CLI implementation used by its standalone srt-* command.")
        .after_help("Examples:\n  vesta flashcards preview --help\n  vesta translate --language-list\n  vesta transcribe run --help\n\nEach command accepts the standalone adapter's options; use its --help for requirements and examples.")
        .subcommand_required(false)
        .arg_required_else_help(false)
        .subcommand(Command::new("flashcards").about("Create or inspect flashcards (srt-flashcards options)"))
        .subcommand(Command::new("translate").about("Translate subtitles (srt-translate options)"))
        .subcommand(Command::new("transcribe").about("Transcribe media to subtitles (srt-transcribe options)"))
        .subcommand(Command::new("sync").about("Retiming subtitles (srt-sync options)"))
        .subcommand(Command::new("autosync").about("Align subtitles to media (srt-autosync options)"))
        .subcommand(Command::new("refine").about("Inspect or annotate flashcard decks (srt-refine options)"))
        .subcommand(Command::new("extract").about("Extract subtitle tracks (srt-extract options)"))
}

/// Dispatch arguments to the existing command implementations without requiring
/// their standalone executables to be installed. `args` follows `args_os()` and
/// therefore includes argv[0].
pub fn run_from(args: Vec<OsString>) -> ExitCode {
    let mut args = args.into_iter();
    let _program = args.next().unwrap_or_else(|| OsString::from("vesta"));
    let Some(operation) = args.next() else {
        let mut cmd = command();
        let _ = cmd.print_help();
        println!();
        return ExitCode::SUCCESS;
    };
    let operation_text = operation.to_string_lossy();
    if operation_text == "help" {
        let mut help_args = vec![OsString::from("vesta")];
        help_args.extend(args);
        help_args.push(OsString::from("--help"));
        return run_from(help_args);
    }

    if operation_text == "--help" || operation_text == "-h" {
        let mut cmd = command();
        let _ = cmd.print_help();
        println!();
        return ExitCode::SUCCESS;
    }
    if operation_text == "--version" || operation_text == "-V" {
        println!("{} {}", command().get_name(), env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    let command_name = match operation_text.as_ref() {
        "flashcards" => "srt-flashcards",
        "translate" => "srt-translate",
        "transcribe" => "srt-transcribe",
        "sync" => "srt-sync",
        "autosync" => "srt-autosync",
        "refine" => "srt-refine",
        "extract" => "srt-extract",
        unknown => {
            let error = command().error(
                ErrorKind::InvalidSubcommand,
                format!("unrecognized operation '{unknown}'"),
            );
            let exit_code = error.exit_code() as u8;
            let _ = error.print();
            return ExitCode::from(exit_code);
        }
    };
    let forwarded = std::iter::once(OsString::from(command_name))
        .chain(args)
        .collect();

    match command_name {
        "srt-flashcards" => srt_flashcards_cli::run_from(forwarded),
        "srt-translate" => srt_translate_cli::run_from(forwarded),
        "srt-transcribe" => srt_transcribe_cli::run_from(forwarded),
        "srt-sync" => match srt_sync_cli::run_from(forwarded) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("srt-sync: {error:#}");
                ExitCode::FAILURE
            }
        },
        "srt-autosync" => srt_autosync_cli::run_from(forwarded),
        "srt-refine" => srt_refine_cli::run_from(forwarded),
        "srt-extract" => match srt_extract_cli::run_from(forwarded) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("srt-extract: {error:#}");
                ExitCode::FAILURE
            }
        },
        _ => unreachable!("operation mapped above"),
    }
}

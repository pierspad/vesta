fn main() -> std::process::ExitCode {
    srt_flashcards_cli::run_from(std::env::args_os().collect())
}

fn main() -> std::process::ExitCode {
    srt_transcribe_cli::run_from(std::env::args_os().collect())
}

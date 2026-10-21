fn main() -> std::process::ExitCode {
    srt_translate_cli::run_from(std::env::args_os().collect())
}

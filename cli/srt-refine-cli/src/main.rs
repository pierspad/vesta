fn main() -> std::process::ExitCode {
    srt_refine_cli::run_from(std::env::args_os().collect())
}

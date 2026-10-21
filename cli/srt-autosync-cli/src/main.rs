fn main() -> std::process::ExitCode {
    srt_autosync_cli::run_from(std::env::args_os().collect())
}

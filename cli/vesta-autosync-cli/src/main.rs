fn main() -> std::process::ExitCode {
    vesta_autosync_cli::run_from(std::env::args_os().collect())
}

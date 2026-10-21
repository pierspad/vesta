fn main() -> std::process::ExitCode {
    vesta_cli::run_from(std::env::args_os().collect())
}

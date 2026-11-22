fn main() -> std::process::ExitCode {
    vesta_translate_cli::run_from(std::env::args_os().collect())
}

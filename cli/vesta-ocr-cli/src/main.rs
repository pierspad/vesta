fn main() -> std::process::ExitCode {
    vesta_ocr_cli::run_from(std::env::args_os().collect())
}

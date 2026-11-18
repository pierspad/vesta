fn main() -> std::process::ExitCode {
    srt_ocr_cli::run_from(std::env::args_os().collect())
}

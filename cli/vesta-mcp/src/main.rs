fn main() -> std::process::ExitCode {
    vesta_mcp::run_from(std::env::args_os().collect())
}

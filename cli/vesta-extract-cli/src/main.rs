fn main() -> anyhow::Result<()> {
    vesta_extract_cli::run_from(std::env::args_os().collect())
}

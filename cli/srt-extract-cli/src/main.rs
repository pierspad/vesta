fn main() -> anyhow::Result<()> {
    srt_extract_cli::run_from(std::env::args_os().collect())
}

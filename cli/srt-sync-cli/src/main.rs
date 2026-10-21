fn main() -> anyhow::Result<()> {
    srt_sync_cli::run_from(std::env::args_os().collect())
}

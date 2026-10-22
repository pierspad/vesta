use clap::Parser;
use rmcp::ServiceExt;
use std::path::PathBuf;
use vesta_mcp::{Paths, ProviderConfig, VestaMcp};

#[derive(Parser)]
#[command(
    version,
    about = "Local MCP server for Vesta (stdio, no GUI or listening port)",
    after_help = "Example:\n  vesta-mcp --read-root /absolute/path/to/media --output-root /absolute/path/to/exports\n\nBoth roots must already exist. Repeat --read-root for more input directories.\nStart this command from an MCP client: stdin/stdout carry protocol messages, not an interactive prompt.\nTools cover subtitle inspection, previews and cancellable flashcard exports; translation/transcription require --provider-config.\nUse vesta-cli <operation> --help for terminal workflows."
)]
struct Args {
    /// Existing input directory; repeat to allow more directories.
    #[arg(long, required = true)]
    read_root: Vec<PathBuf>,
    /// Existing directory for fresh per-job output subdirectories.
    #[arg(long)]
    output_root: PathBuf,
    /// Optional server-side JSON profiles; credentials are resolved from named environment variables.
    #[arg(long)]
    provider_config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let providers = args
        .provider_config
        .map(ProviderConfig::load)
        .transpose()
        .map_err(anyhow::Error::msg)?
        .unwrap_or_default();
    let server =
        VestaMcp::with_provider_config(Paths::new(args.read_root, args.output_root)?, providers);
    let jobs = server.jobs();
    // stdout belongs exclusively to the SDK's JSON-RPC transport.
    let service = server.serve(rmcp::transport::stdio()).await?;
    let cancel = service.cancellation_token();
    let finished = service.waiting();
    tokio::pin!(finished);
    let result = tokio::select! {
        result = &mut finished => result,
        _ = tokio::signal::ctrl_c() => { cancel.cancel(); finished.await }
    };
    jobs.shutdown().await;
    result?;
    Ok(())
}

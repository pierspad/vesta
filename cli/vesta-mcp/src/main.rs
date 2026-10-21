use clap::Parser;
use rmcp::ServiceExt;
use std::path::PathBuf;
use vesta_mcp::{Paths, VestaMcp};

#[derive(Parser)]
#[command(
    version,
    about = "Local MCP server for Vesta (stdio, no GUI or listening port)"
)]
struct Args {
    /// Existing input directory; repeat to allow more directories.
    #[arg(long, required = true)]
    read_root: Vec<PathBuf>,
    /// Existing directory for fresh per-job output subdirectories.
    #[arg(long)]
    output_root: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let server = VestaMcp::new(Paths::new(args.read_root, args.output_root)?);
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

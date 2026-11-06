use rmcp::{ServiceExt, transport::stdio};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("peachy-mcp: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let usage = "Usage: peachy-mcp --root /absolute/workspace\nServes a separate headless editor session over MCP stdio. Writes only fresh outputs within --root; no GUI bridge.";
    let first = args.next();
    if first.as_deref() == Some(std::ffi::OsStr::new("--help")) {
        eprintln!("{usage}");
        return Ok(());
    }
    if first.as_deref() != Some(std::ffi::OsStr::new("--root")) {
        return Err(usage.into());
    }
    let root = PathBuf::from(args.next().ok_or(usage)?);
    if args.next().is_some() || !root.is_absolute() {
        return Err(usage.into());
    }
    let server = peachy_mcp::AnimatorServer::new(&root)?;
    let service = server
        .serve(stdio())
        .await
        .map_err(|error| error.to_string())?;
    service.waiting().await.map_err(|error| error.to_string())?;
    Ok(())
}

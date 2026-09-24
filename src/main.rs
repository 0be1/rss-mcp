// SPDX-License-Identifier: GPL-3.0-or-later
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use tracing_subscriber::EnvFilter;

use rss_mcp::config::Config;
use rss_mcp::server::RssServer;

/// MCP server exposing RSS/Atom feeds over stdio.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Path to the TOML configuration file.
    #[arg(short, long, env = "RSS_MCP_CONFIG")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // stdout carries the JSON-RPC protocol: logs MUST go to stderr.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("rss_mcp=info".parse()?))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let args = Args::parse();
    let config = Config::load(&args.config)
        .with_context(|| format!("failed to load {}", args.config.display()))?;
    tracing::info!(feeds = config.feeds.len(), "configuration loaded");

    let service = RssServer::new(config)?
        .serve(stdio())
        .await
        .context("failed to start MCP server")?;
    service.waiting().await?;
    Ok(())
}

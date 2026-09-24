//! MCP server: exposes the configured feeds as tools.
//!
//! This is a thin adapter layer. Business logic lives in [`crate::feed`] and
//! [`crate::config`]; this module only maps it to MCP tool calls.

use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Json;
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use serde::Serialize;

use crate::config::Config;
use crate::feed::{FeedClient, FeedError};

/// Public description of a configured feed.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FeedInfo {
    pub name: String,
    pub url: String,
    pub tags: Vec<String>,
}

/// Result of the `list_feeds` tool.
///
/// Structured tool output must be a JSON object, hence this wrapper.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FeedList {
    pub feeds: Vec<FeedInfo>,
}

#[derive(Debug, Clone)]
pub struct RssServer {
    config: Arc<Config>,
    // Not used yet: will serve the upcoming `get_feed` tool.
    #[allow(dead_code)]
    client: FeedClient,
    tool_router: ToolRouter<Self>,
}

impl RssServer {
    pub fn new(config: Config) -> Result<Self, FeedError> {
        let client = FeedClient::new(config.settings.timeout())?;
        Ok(Self {
            config: Arc::new(config),
            client,
            tool_router: Self::tool_router(),
        })
    }
}

#[tool_router]
impl RssServer {
    #[tool(
        description = "List the RSS/Atom feeds available on this server, with their name, URL and tags."
    )]
    pub async fn list_feeds(&self) -> Json<FeedList> {
        let feeds = self
            .config
            .feeds
            .iter()
            .map(|f| FeedInfo {
                name: f.name.clone(),
                url: f.url.clone(),
                tags: f.tags.clone(),
            })
            .collect();
        Json(FeedList { feeds })
    }
}

#[tool_handler(
    router = self.tool_router,
    name = "rss-mcp",
    instructions = "Reads RSS and Atom feeds declared in a TOML configuration file. Call `list_feeds` to discover the available feeds."
)]
impl ServerHandler for RssServer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_feeds_returns_configured_feeds() {
        let config: Config = r#"
            [[feeds]]
            name = "a"
            url = "https://example.com/a.xml"
            tags = ["x"]
            [[feeds]]
            name = "b"
            url = "https://example.com/b.xml"
        "#
        .parse()
        .unwrap();

        let Json(list) = RssServer::new(config).unwrap().list_feeds().await;

        let names: Vec<_> = list.feeds.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["a", "b"]);
        assert_eq!(list.feeds[0].tags, ["x"]);
    }
}

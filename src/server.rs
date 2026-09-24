//! MCP server: exposes the configured feeds as tools.
//!
//! This is a thin adapter layer. Business logic lives in [`crate::feed`] and
//! [`crate::config`]; this module only maps it to MCP tool calls.

use std::sync::Arc;

use rmcp::{
    ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    tool, tool_handler, tool_router,
};
use serde::Serialize;

use crate::config::Config;
use crate::feed::{FeedClient, FeedError, FeedItem};

/// Public description of a configured feed.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FeedInfo {
    pub name: String,
    pub url: String,
    pub tags: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
/// Parameters of the `get_feed` tool.
pub struct FeedRequest {
    /// Feed name, as returned by `list_feeds`.
    pub name: String,
    /// Maximum number of items to return (defaults to the server setting).
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FeedResponse {
    pub items: Vec<FeedItem>,
}

/// Result of the `list_feeds` tool.
/// Structured tool output must be a JSON object, hence this wrapper.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FeedList {
    pub feeds: Vec<FeedInfo>,
}

#[derive(Debug, Clone)]
pub struct RssServer {
    config: Arc<Config>,
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

    #[tool(
        description = "Fetch a feed by name and return its most recent items (title, link, publication date, summary). Use list_feeds to discover valid names."
    )]
    pub async fn get_feed(
        &self,
        Parameters(args): Parameters<FeedRequest>,
    ) -> Result<Json<FeedResponse>, String> {
        let Some(feed) = self.config.feed(&args.name) else {
            return Err(format!(
                "unknown feed {:?}; call list_feeds to see available names",
                args.name
            ));
        };

        let limit = args.limit.unwrap_or(self.config.settings.max_items);

        let mut items = self.client.fetch(&feed.url).await.map_err(|e| {
            format!(
                "cannot read feed {:?}: {:#}",
                feed.name,
                anyhow::Error::from(e)
            )
        })?;

        items.truncate(limit);

        Ok(Json(FeedResponse { items }))
    }
}

#[tool_handler(
    router = self.tool_router,
    name = "rss-mcp",
    instructions = "Reads RSS and Atom feeds declared in a TOML configuration file. Call `list_feeds` to discover the available feeds."
)]
impl ServerHandler for RssServer {}

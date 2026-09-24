//! Fetching and parsing of RSS/Atom documents.
//!
//! This module knows nothing about MCP: it turns a URL (or raw bytes) into a
//! list of [`FeedItem`]s, which keeps it easy to test in isolation.

use std::time::Duration;

use serde::Serialize;

/// Errors that can occur while retrieving or parsing a feed.
#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    #[error("HTTP request failed")]
    Http(#[from] reqwest::Error),

    #[error("cannot parse feed")]
    Parse(#[from] feed_rs::parser::ParseFeedError),
}

/// A simplified, format-agnostic view of a feed entry.
#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
pub struct FeedItem {
    pub title: Option<String>,
    /// First link of the entry, usually the article URL.
    pub link: Option<String>,
    /// Publication date (or last update date as a fallback), in RFC 3339 format.
    pub published: Option<String>,
    pub summary: Option<String>,
}

/// Parses an RSS or Atom document and returns its items in document order.
pub fn parse_feed(bytes: &[u8]) -> Result<Vec<FeedItem>, FeedError> {
    let feed = feed_rs::parser::parse(bytes)?;
    let items = feed
        .entries
        .into_iter()
        .map(|entry| FeedItem {
            title: entry.title.map(|t| t.content),
            link: entry.links.into_iter().next().map(|l| l.href),
            published: entry.published.or(entry.updated).map(|d| d.to_rfc3339()),
            summary: entry.summary.map(|t| t.content),
        })
        .collect();
    Ok(items)
}

/// HTTP client dedicated to downloading feeds.
#[derive(Debug, Clone)]
pub struct FeedClient {
    http: reqwest::Client,
}

impl FeedClient {
    pub fn new(timeout: Duration) -> Result<Self, FeedError> {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        Ok(Self { http })
    }

    /// Downloads the document at `url` and parses it.
    pub async fn fetch(&self, url: &str) -> Result<Vec<FeedItem>, FeedError> {
        let bytes = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        parse_feed(&bytes)
    }
}

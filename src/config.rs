//! Configuration loading from a TOML file.
//!
//! The file contains an optional `[settings]` table and a list of `[[feeds]]`.
//! See `feeds.example.toml` at the repository root for a complete example.

use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use reqwest::Url;
use serde::Deserialize;

/// Errors that can occur while loading or validating the configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read config file {path}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid TOML")]
    Toml(#[from] toml::de::Error),

    #[error("the config must declare at least one [[feeds]] entry")]
    NoFeeds,

    #[error("duplicate feed name: {0:?}")]
    DuplicateName(String),

    #[error("feed {name:?} has an invalid URL {url:?}: {reason}")]
    InvalidUrl {
        name: String,
        url: String,
        reason: String,
    },
}

/// Root of the configuration file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub feeds: Vec<FeedConfig>,
}

/// Global settings shared by all feeds.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// HTTP timeout, in seconds, for a single feed request.
    pub timeout_secs: u64,
    /// Default maximum number of items returned per feed.
    pub max_items: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            timeout_secs: 10,
            max_items: 20,
        }
    }
}

impl Settings {
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }
}

/// A single feed declared in the configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedConfig {
    /// Unique, human-friendly identifier used by the MCP tools.
    pub name: String,
    /// URL of the RSS or Atom document.
    pub url: String,
    /// Free-form labels used to group feeds.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Config {
    /// Reads, parses and validates the configuration file at `path`.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.display().to_string(),
            source,
        })?;
        text.parse()
    }

    /// Checks invariants that serde cannot express.
    fn validate(&self) -> Result<(), ConfigError> {
        if self.feeds.is_empty() {
            return Err(ConfigError::NoFeeds);
        }

        let mut seen = HashSet::new();
        for feed in &self.feeds {
            if !seen.insert(feed.name.as_str()) {
                return Err(ConfigError::DuplicateName(feed.name.clone()));
            }

            let invalid = |reason: String| ConfigError::InvalidUrl {
                name: feed.name.clone(),
                url: feed.url.clone(),
                reason,
            };
            let url = Url::parse(&feed.url).map_err(|e| invalid(e.to_string()))?;
            if !matches!(url.scheme(), "http" | "https") {
                return Err(invalid("only http and https are supported".into()));
            }
        }
        Ok(())
    }

    /// Looks up a feed by its name.
    pub fn feed(&self, name: &str) -> Option<&FeedConfig> {
        self.feeds.iter().find(|f| f.name == name)
    }
}

impl std::str::FromStr for Config {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config: Config = toml::from_str(s)?;
        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_config() {
        let config: Config = r#"
            [settings]
            timeout_secs = 5
            max_items = 3

            [[feeds]]
            name = "rust-blog"
            url = "https://blog.rust-lang.org/feed.xml"
            tags = ["rust"]
        "#
        .parse()
        .unwrap();

        assert_eq!(config.settings.timeout_secs, 5);
        assert_eq!(config.settings.max_items, 3);
        assert_eq!(config.feeds.len(), 1);
        assert_eq!(config.feeds[0].tags, ["rust"]);
        assert!(config.feed("rust-blog").is_some());
        assert!(config.feed("unknown").is_none());
    }

    #[test]
    fn settings_and_tags_are_optional() {
        let config: Config = r#"
            [[feeds]]
            name = "a"
            url = "http://example.com/rss"
        "#
        .parse()
        .unwrap();

        assert_eq!(config.settings.timeout_secs, 10);
        assert_eq!(config.settings.max_items, 20);
        assert!(config.feeds[0].tags.is_empty());
    }

    #[test]
    fn rejects_empty_feed_list() {
        let err = "".parse::<Config>().unwrap_err();
        assert!(matches!(err, ConfigError::NoFeeds));
    }

    #[test]
    fn rejects_duplicate_names() {
        let err = r#"
            [[feeds]]
            name = "a"
            url = "https://example.com/1"
            [[feeds]]
            name = "a"
            url = "https://example.com/2"
        "#
        .parse::<Config>()
        .unwrap_err();
        assert!(matches!(err, ConfigError::DuplicateName(name) if name == "a"));
    }

    #[test]
    fn rejects_invalid_urls() {
        for url in ["not a url", "ftp://example.com/feed"] {
            let toml = format!("[[feeds]]\nname = \"a\"\nurl = \"{url}\"");
            let err = toml.parse::<Config>().unwrap_err();
            assert!(matches!(err, ConfigError::InvalidUrl { .. }), "{url}");
        }
    }

    #[test]
    fn rejects_unknown_fields() {
        let err = r#"
            [[feeds]]
            name = "a"
            url = "https://example.com"
            typo = true
        "#
        .parse::<Config>()
        .unwrap_err();
        assert!(matches!(err, ConfigError::Toml(_)));
    }
}

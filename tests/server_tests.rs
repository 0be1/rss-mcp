//! Tests for the MCP tools exposed by `RssServer`.

use rmcp::handler::server::wrapper::{Json, Parameters};
use rss_mcp::config::Config;
use rss_mcp::server::{FeedRequest, RssServer};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Contains two items.
const RSS2: &[u8] = include_bytes!("fixtures/rss2.xml");

/// Builds a server with a single feed named "test" pointing at `url`.
fn server_with_feed(url: &str, max_items: usize) -> RssServer {
    let config: Config = format!(
        r#"
        [settings]
        max_items = {max_items}

        [[feeds]]
        name = "test"
        url = "{url}"
        "#
    )
    .parse()
    .unwrap();
    RssServer::new(config).unwrap()
}

/// Starts a mock HTTP server answering `/feed.xml` with `response`.
async fn mock_feed(response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

/// Like `unwrap_err`, which cannot be used here because `rmcp::Json` does not implement `Debug`.
fn expect_err<T>(result: Result<T, String>) -> String {
    match result {
        Ok(_) => panic!("expected an error, got a successful result"),
        Err(err) => err,
    }
}

fn request(name: &str, limit: Option<usize>) -> Parameters<FeedRequest> {
    Parameters(FeedRequest {
        name: name.to_string(),
        limit,
    })
}

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

#[tokio::test]
async fn get_feed_returns_items() {
    let mock = mock_feed(ResponseTemplate::new(200).set_body_bytes(RSS2)).await;
    let server = server_with_feed(&format!("{}/feed.xml", mock.uri()), 20);

    let Json(response) = server.get_feed(request("test", None)).await.unwrap();

    let titles: Vec<_> = response.items.iter().map(|i| i.title.as_deref()).collect();
    assert_eq!(titles, [Some("First post"), Some("Second post")]);
}

#[tokio::test]
async fn get_feed_applies_requested_limit() {
    let mock = mock_feed(ResponseTemplate::new(200).set_body_bytes(RSS2)).await;
    let server = server_with_feed(&format!("{}/feed.xml", mock.uri()), 20);

    let Json(response) = server.get_feed(request("test", Some(1))).await.unwrap();

    assert_eq!(response.items.len(), 1);
    // The limit keeps the first items of the document, not arbitrary ones.
    assert_eq!(response.items[0].title.as_deref(), Some("First post"));
}

#[tokio::test]
async fn get_feed_defaults_to_max_items_setting() {
    let mock = mock_feed(ResponseTemplate::new(200).set_body_bytes(RSS2)).await;
    let server = server_with_feed(&format!("{}/feed.xml", mock.uri()), 1);

    let Json(response) = server.get_feed(request("test", None)).await.unwrap();

    assert_eq!(response.items.len(), 1);
}

#[tokio::test]
async fn get_feed_rejects_unknown_names() {
    // The URL is never contacted: the name lookup fails first.
    let server = server_with_feed("http://127.0.0.1:9/unused.xml", 20);

    let err = expect_err(server.get_feed(request("nope", None)).await);

    assert!(err.contains("\"nope\""), "{err}");
    assert!(err.contains("list_feeds"), "{err}");
}

#[tokio::test]
async fn get_feed_reports_fetch_errors_with_their_cause() {
    let mock = mock_feed(ResponseTemplate::new(404)).await;
    let server = server_with_feed(&format!("{}/feed.xml", mock.uri()), 20);

    let err = expect_err(server.get_feed(request("test", None)).await);

    assert!(err.contains("\"test\""), "{err}");
    // The full error chain is kept, down to the HTTP status.
    assert!(err.contains("404"), "{err}");
}

#[tokio::test]
async fn get_feed_reports_parse_errors() {
    let mock =
        mock_feed(ResponseTemplate::new(200).set_body_string("<html>Not a feed</html>")).await;
    let server = server_with_feed(&format!("{}/feed.xml", mock.uri()), 20);

    let err = expect_err(server.get_feed(request("test", None)).await);

    assert!(err.contains("cannot parse feed"), "{err}");
}

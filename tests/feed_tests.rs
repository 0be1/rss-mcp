//! Tests for feed parsing (local fixtures) and fetching (mock HTTP server).

use std::time::Duration;

use rss_mcp::feed::{FeedClient, FeedError, parse_feed};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const RSS2: &[u8] = include_bytes!("fixtures/rss2.xml");
const ATOM: &[u8] = include_bytes!("fixtures/atom.xml");

#[test]
fn parses_rss2() {
    let items = parse_feed(RSS2).unwrap();

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].title.as_deref(), Some("First post"));
    assert_eq!(items[0].link.as_deref(), Some("https://example.com/first"));
    assert_eq!(items[0].summary.as_deref(), Some("Hello from RSS"));
    assert_eq!(
        items[0].published.as_deref(),
        Some("2025-09-01T10:00:00+00:00")
    );
    // Optional fields stay empty rather than failing the whole feed.
    assert_eq!(items[1].published, None);
    assert_eq!(items[1].summary, None);
}

#[test]
fn parses_atom() {
    let items = parse_feed(ATOM).unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title.as_deref(), Some("Atom entry"));
    assert_eq!(
        items[0].link.as_deref(),
        Some("https://example.com/atom-entry")
    );
    // Atom entry without <published>: falls back to <updated>.
    assert_eq!(
        items[0].published.as_deref(),
        Some("2025-09-02T12:00:00+00:00")
    );
}

#[test]
fn rejects_non_feed_documents() {
    let err = parse_feed(b"<html><body>Not a feed</body></html>").unwrap_err();
    assert!(matches!(err, FeedError::Parse(_)));
}

#[tokio::test]
async fn fetches_feed_over_http() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(ATOM))
        .mount(&server)
        .await;

    let client = FeedClient::new(Duration::from_secs(5)).unwrap();
    let items = client
        .fetch(&format!("{}/feed.xml", server.uri()))
        .await
        .unwrap();

    assert_eq!(items.len(), 1);
}

#[tokio::test]
async fn reports_http_errors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = FeedClient::new(Duration::from_secs(5)).unwrap();
    let err = client.fetch(&server.uri()).await.unwrap_err();

    assert!(matches!(err, FeedError::Http(e) if e.status().map(|s| s.as_u16()) == Some(404)));
}

#[tokio::test]
async fn times_out_on_slow_servers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .mount(&server)
        .await;

    let client = FeedClient::new(Duration::from_millis(200)).unwrap();
    let err = client.fetch(&server.uri()).await.unwrap_err();

    assert!(matches!(err, FeedError::Http(e) if e.is_timeout()));
}

#[tokio::test]
async fn identifies_itself_with_a_user_agent() {
    let server = MockServer::start().await;
    // Only answers when the expected User-Agent is sent; otherwise wiremock returns 404.
    Mock::given(method("GET"))
        .and(header(
            "user-agent",
            concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(RSS2))
        .expect(1)
        .mount(&server)
        .await;

    let client = FeedClient::new(Duration::from_secs(5)).unwrap();
    let items = client.fetch(&server.uri()).await.unwrap();

    assert_eq!(items.len(), 2);
}

#[tokio::test]
async fn reports_parse_errors_for_non_feed_responses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>Not a feed</html>"))
        .mount(&server)
        .await;

    let client = FeedClient::new(Duration::from_secs(5)).unwrap();
    let err = client.fetch(&server.uri()).await.unwrap_err();

    assert!(matches!(err, FeedError::Parse(_)));
}

#[tokio::test]
async fn follows_redirects() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/old.xml"))
        .respond_with(
            ResponseTemplate::new(301)
                .insert_header("location", format!("{}/new.xml", server.uri()).as_str()),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/new.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(ATOM))
        .mount(&server)
        .await;

    let client = FeedClient::new(Duration::from_secs(5)).unwrap();
    let items = client
        .fetch(&format!("{}/old.xml", server.uri()))
        .await
        .unwrap();

    assert_eq!(items[0].title.as_deref(), Some("Atom entry"));
}

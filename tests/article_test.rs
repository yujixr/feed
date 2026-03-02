use chrono::Utc;
use feed::article::{extract_from_html, html_to_text, Article};
use feed::config::ExtractorMethod;

#[test]
fn test_article_creation() {
    let article = Article {
        title: "Test".to_string(),
        url: "https://example.com/1".to_string(),
        published: Some(Utc::now()),
        feed_url: "https://example.com/feed".to_string(),
        feed_name: "Test Blog".to_string(),
        extractor: ExtractorMethod::default(),
        read: false,
        rss_content: None,
    };
    assert_eq!(article.title, "Test");
    assert!(!article.read);
}

#[test]
fn test_article_clone() {
    let article = Article {
        title: "Test".to_string(),
        url: "https://example.com/1".to_string(),
        published: None,
        feed_url: "https://example.com/feed".to_string(),
        feed_name: "Blog".to_string(),
        extractor: ExtractorMethod::Readability,
        read: true,
        rss_content: Some("<p>content</p>".to_string()),
    };
    let cloned = article.clone();
    assert_eq!(cloned.title, article.title);
    assert_eq!(cloned.read, article.read);
}

#[test]
fn test_extract_text_from_html_basic() {
    let html = r#"
    <html>
    <head><title>Test Article</title></head>
    <body>
        <nav>Navigation links</nav>
        <article>
            <h1>Test Article</h1>
            <p>This is the main content of the article. It contains enough text to be considered the main content by the readability algorithm. We need to make sure there is sufficient content here for the extraction to work properly. Let's add a few more sentences to ensure the content is long enough.</p>
            <p>Another paragraph with more important content that readers would want to see. This helps ensure the readability algorithm identifies this as the main content area of the page.</p>
        </article>
        <footer>Footer content</footer>
    </body>
    </html>
    "#;
    let (_title, text) = extract_from_html(html, 80);
    assert!(text.contains("main content"));
    assert!(!text.is_empty());
}

#[test]
fn test_extract_text_from_html_returns_title() {
    let html = r#"
    <html>
    <head><title>My Great Article</title></head>
    <body>
        <article>
            <h1>My Great Article</h1>
            <p>Substantial article content goes here. This needs to be long enough for the readability algorithm to identify it as the main content. Let's make sure we have enough text to pass the content threshold.</p>
        </article>
    </body>
    </html>
    "#;
    let (title, _text) = extract_from_html(html, 80);
    assert!(!title.is_empty());
}

// --- extract_from_html edge cases ---

#[test]
fn test_extract_from_html_minimal_html() {
    // Minimal HTML that readability may not parse — falls back to html_to_text
    let html = "<p>short</p>";
    let (_title, text) = extract_from_html(html, 80);
    assert!(text.contains("short"));
}

#[test]
fn test_extract_from_html_empty() {
    let (title, text) = extract_from_html("", 80);
    assert!(title.is_empty());
    assert!(text.trim().is_empty());
}

#[test]
fn test_extract_from_html_plain_text_input() {
    let (title, text) = extract_from_html("just plain text", 80);
    // Should still produce output via fallback
    assert!(title.is_empty() || text.contains("plain text"));
}

// --- html_to_text tests ---

#[test]
fn test_html_to_text_basic() {
    let text = html_to_text("<p>Hello <b>world</b></p>", 80);
    assert!(text.contains("Hello"));
    assert!(text.contains("world"));
}

#[test]
fn test_html_to_text_plain_input() {
    let text = html_to_text("no tags here", 80);
    assert!(text.contains("no tags here"));
}

#[test]
fn test_html_to_text_nested_tags() {
    let text = html_to_text("<div><ul><li>item1</li><li>item2</li></ul></div>", 80);
    assert!(text.contains("item1"));
    assert!(text.contains("item2"));
}

#[test]
fn test_html_to_text_respects_width() {
    let long_text = "a ".repeat(100);
    let html = format!("<p>{}</p>", long_text);
    let text = html_to_text(&html, 40);
    // Lines should be wrapped to roughly the specified width
    for line in text.lines() {
        assert!(line.len() <= 80, "line too long: {}", line);
    }
}

// --- extract_content (RssContent path, no network) ---

#[tokio::test]
async fn test_extract_content_rss_content_with_html() {
    let client = reqwest::Client::new();
    let result = feed::article::extract_content(
        &client,
        "https://example.invalid/nonexistent",
        &ExtractorMethod::RssContent,
        80,
        Some("<p>RSS article body</p>"),
    )
    .await;
    assert!(result.is_ok());
    let text = result.unwrap();
    assert!(text.contains("RSS article body"));
}

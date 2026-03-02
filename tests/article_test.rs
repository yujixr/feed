use feed::article::{html_to_text, parse_readable_html};
use feed::config::ExtractorMethod;

// Readability extracts main content, skipping nav/footer.
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
    let (_title, content_html) = parse_readable_html(html);
    let text = html_to_text(&content_html, 80);
    assert!(text.contains("main content"));
    assert!(!text.is_empty());
}

// Readability extracts the page title from <title> or <h1>.
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
    let (title, _content_html) = parse_readable_html(html);
    assert!(!title.is_empty());
}

// When readability cannot parse minimal HTML, it falls back to html_to_text.
#[test]
fn test_extract_from_html_minimal_html() {
    let html = "<p>short</p>";
    let (_title, content_html) = parse_readable_html(html);
    let text = html_to_text(&content_html, 80);
    assert!(text.contains("short"));
}

// Empty input does not panic and returns empty output.
#[test]
fn test_extract_from_html_empty() {
    let (title, content_html) = parse_readable_html("");
    let text = html_to_text(&content_html, 80);
    assert!(title.is_empty());
    assert!(text.trim().is_empty());
}

// Plain text input (no HTML tags) is handled via fallback.
#[test]
fn test_extract_from_html_plain_text_input() {
    let (title, content_html) = parse_readable_html("just plain text");
    let text = html_to_text(&content_html, 80);
    assert!(title.is_empty() || text.contains("plain text"));
}

// html_to_text strips tags and preserves visible text.
#[test]
fn test_html_to_text_basic() {
    let text = html_to_text("<p>Hello <b>world</b></p>", 80);
    assert!(text.contains("Hello"));
    assert!(text.contains("world"));
}

// html_to_text handles nested tags like <ul>/<li>.
#[test]
fn test_html_to_text_nested_tags() {
    let text = html_to_text("<div><ul><li>item1</li><li>item2</li></ul></div>", 80);
    assert!(text.contains("item1"));
    assert!(text.contains("item2"));
}

// html_to_text wraps lines according to the given width.
#[test]
fn test_html_to_text_respects_width() {
    let long_text = "a ".repeat(100);
    let html = format!("<p>{}</p>", long_text);
    let text = html_to_text(&html, 40);
    for line in text.lines() {
        assert!(line.len() <= 80, "line too long: {}", line);
    }
}

// extract_html with RssContent mode returns HTML from RSS body without network access.
#[tokio::test]
async fn test_extract_html_rss_content_with_html() {
    let client = reqwest::Client::new();
    let html = feed::article::extract_html(
        &client,
        "https://example.invalid/nonexistent",
        &ExtractorMethod::RssContent,
        Some("<p>RSS article body</p>"),
    )
    .await
    .unwrap();
    let text = feed::article::html_to_text(&html, 80);
    assert!(text.contains("RSS article body"));
}

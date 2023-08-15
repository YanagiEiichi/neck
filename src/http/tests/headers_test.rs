use tokio;

use super::StringWritter;

use super::super::Headers;

#[test]
fn test_collect() {
    let mut headers: Headers = vec!["a: 1", "b: 2"].iter().map(|s| s.to_string()).collect();

    assert_eq!(headers.len(), 2);
    assert_eq!(headers.get_header("a"), Some("1"));
    assert_eq!(headers.get_header("b"), Some("2"));
    assert_eq!(headers.get_header("c"), None);

    assert_eq!(
        {
            let s = String::from("a");
            let k = s.as_str();
            headers.get_header(k)
        },
        Some("1")
    );

    assert_eq!(headers.remove("b"), Some("b: 2".to_string().into()));

    assert_eq!(headers.len(), 1);
    assert_eq!(headers.get_header("a"), Some("1"));
    assert_eq!(headers.get_header("b"), None);
}

#[tokio::test]
async fn test_write() {
    let headers: Headers = vec!["a: 1", "b: 2"].iter().map(|s| s.to_string()).collect();
    let mut w = StringWritter::new();
    headers.write_to(&mut w).await.unwrap();
    assert_eq!(w.to_string(), String::from("a: 1\r\nb: 2\r\n"));
}

use tokio;

use super::StringWritter;

use super::super::FirstLine;

#[test]
fn test_request() {
    let fl: FirstLine = "GET /api HTTP/1.1".to_string().try_into().unwrap();
    assert_eq!(fl.get_first(), "GET");
    assert_eq!(fl.get_second(), "/api");
    assert_eq!(fl.get_third(), "HTTP/1.1");
}

#[test]
fn test_response() {
    let fl: FirstLine = "HTTP/1.1 400 Bad Request".to_string().try_into().unwrap();
    assert_eq!(fl.get_first(), "HTTP/1.1");
    assert_eq!(fl.get_second(), "400");
    assert_eq!(fl.get_third(), "Bad Request");
}

#[tokio::test]
async fn test_new() {
    let fl = FirstLine::new("HEAD", "/%20/a+b/gg", "HTTP/1.0");
    assert_eq!(fl.get_first(), "HEAD");
    assert_eq!(fl.get_second(), "/%20/a+b/gg");
    assert_eq!(fl.get_third(), "HTTP/1.0");

    let mut w = StringWritter::new();
    fl.write_to(&mut w).await.unwrap();
    assert_eq!(w.to_string(), String::from("HEAD /%20/a+b/gg HTTP/1.0\r\n"));
}

#[tokio::test]
async fn test_write_to() {
    let fl: FirstLine = "GET /api HTTP/1.1".to_string().try_into().unwrap();
    let mut w = StringWritter::new();
    fl.write_to(&mut w).await.unwrap();
    assert_eq!(w.to_string(), String::from("GET /api HTTP/1.1\r\n"));
}

use std::{collections::HashMap, sync::Once};

use crate::{http::{HttpRequest, HttpResponse}, utils::NeckStream};

pub struct StaticMatcher(HashMap<&'static str, &'static [u8]>);

impl StaticMatcher {
    pub fn new() -> Self {
        StaticMatcher(HashMap::new())
    }

    pub fn add_route(mut self, path: &'static str, bytes: &'static [u8]) -> Self {
        self.0.insert(path, bytes);
        self
    }

    fn get_type_by_path(path: &str) -> &'static str {
        match path.rfind('.').map(|p| &path[p + 1..]) {
            Some("js") => "application/javascript",
            Some("html") => "text/html",
            None => "text/html",
            _ => "text/plain",
        }
    }

    pub async fn execute(&self, req: &HttpRequest, stream: &NeckStream) -> std::io::Result<()> {
        let path = req.get_uri();
        if let Some(bytes) = self.0.get(path) {
            HttpResponse::new(200, "OK", req.get_version())
                .add_payload(bytes)
                .add_header_kv("Content-Type", StaticMatcher::get_type_by_path(path))
                .add_header("Cache-Control: no-cache")
                .write_to_stream(&stream)
                .await
        } else {
            HttpResponse::new(404, "Not Found", req.get_version())
                .add_payload(b"Not Found\n")
                .add_header("Cache-Control: no-cache")
                .write_to_stream(&stream)
                .await
        }
    }
}

pub fn get_static_matcher() -> &'static StaticMatcher {
  static mut CACHE: Option<StaticMatcher> = None;
  static INIT: Once = Once::new();
  INIT.call_once(|| {
      unsafe {
          let matcher = StaticMatcher::new()
              .add_route("/dashboard", include_bytes!("../static/index.html"))
              .add_route("/utils.js", include_bytes!("../static/utils.js"))
              .add_route(
                  "/dataService.js",
                  include_bytes!("../static/dataService.js"),
              );
          CACHE = Some(matcher);
      };
  });
  return unsafe { CACHE.as_ref().unwrap() };
}
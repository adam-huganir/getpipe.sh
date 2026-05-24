use comrak::{markdown_to_html, Options};
use std::collections::BTreeMap;
use std::sync::LazyLock;

const STATIC_FILES: [(&str, &str); 2] = [
  ("index.html", include_str!("../static/index.md")),
  ("404.html", include_str!("../static/404.md")),
];

const HEAD: &str = concat!(
  "<head><title>termlibs</title><style>",
  include_str!("../static/style.css"),
  "</style></head><body>"
);

fn wrap_body(html: &str) -> String {
  format!("{}{}</body>", HEAD, html)
}

static RENDERED_PAGES: LazyLock<BTreeMap<&'static str, String>> = LazyLock::new(|| {
  STATIC_FILES
    .iter()
    .map(|(key, md)| {
      let html = markdown_to_html(md, &Options::default());
      let rendered = wrap_body(&html);
      (*key, rendered)
    })
    .collect()
});

pub(crate) fn load_static(key: &str) -> Option<String> {
  RENDERED_PAGES.get(key).cloned()
}

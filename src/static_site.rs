use comrak::{Options, markdown_to_html};
#[cfg(not(feature = "hot-reload"))]
use std::collections::BTreeMap;
#[cfg(not(feature = "hot-reload"))]
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Static file manifest — key → (disk path, compile-time fallback content).
// Only used by the hot-reload path; the production path uses include_str!.
// The fallback keeps integration tests working when GETPIPE_ROOT is
// temporarily redirected (e.g. by a concurrent hot-reload unit test).
// ---------------------------------------------------------------------------
#[cfg(feature = "hot-reload")]
const STATIC_FILE_MAPPING: [(&str, &str, &str); 2] = [
  (
    "index.html",
    "static/index.md",
    include_str!("../static/index.md"),
  ),
  (
    "404.html",
    "static/404.md",
    include_str!("../static/404.md"),
  ),
];


fn comrak_options() -> Options<'static> {
  let mut opts = Options::default();
  opts.extension.table = true;
  opts.extension.strikethrough = true;
  opts.extension.alerts = true;
  opts.extension.shortcodes = true;
  opts.extension.underline = true;
  opts.extension.highlight = true;
  opts.extension.block_directive = true;
  opts
}

const HEAD: &str = concat!(
  "<head><title>getpipe.sh</title><style>",
  include_str!("../static/css/pico.classless.purple.css"),
  include_str!("../static/css/overrides.css"),
  "</style></head><body>"
);

const CLIPBOARD_JS: &str = include_str!("../static/clipboard.js");

fn wrap_body(html: &str) -> String {
  #[cfg(not(feature = "hot-reload"))]
  let head_str: &str = HEAD;
  #[cfg(feature = "hot-reload")]
  let head_str = format!(
    "<head><title>getpipe.sh</title><style>{}{}</style></head><body>",
    crate::hot_reload::read("static/css/pico.classless.purple.css", ""),
    crate::hot_reload::read("static/css/overrides.css", ""),
  );

  #[cfg(not(feature = "hot-reload"))]
  let clipboard_js: &str = CLIPBOARD_JS;
  #[cfg(feature = "hot-reload")]
  let clipboard_js = crate::hot_reload::read("static/clipboard.js", CLIPBOARD_JS);

  #[cfg(not(feature = "hot-reload"))]
  let (copy_svg, check_svg): (&str, &str) = (
    include_str!("../static/icons/copy.svg"),
    include_str!("../static/icons/check.svg"),
  );
  #[cfg(feature = "hot-reload")]
  let (copy_svg, check_svg) = (
    crate::hot_reload::read("static/icons/copy.svg", ""),
    crate::hot_reload::read("static/icons/check.svg", ""),
  );

  format!(
    "{}<span hidden id=\"icon-copy\">{}</span><span hidden id=\"icon-check\">{}</span><main>{}</main><script>{}</script></body>",
    head_str,
    copy_svg.trim(),
    check_svg.trim(),
    html,
    clipboard_js,
  )
}

fn render_md(md: &str) -> String {
  wrap_body(&markdown_to_html(md, &comrak_options()))
}

// ---------------------------------------------------------------------------
// Production path — static files embedded at compile time.
// ---------------------------------------------------------------------------
#[cfg(not(feature = "hot-reload"))]
static RENDERED_PAGES: LazyLock<BTreeMap<&'static str, String>> = LazyLock::new(|| {
  let sources: [(&str, &str); 2] = [
    ("index.html", include_str!("../static/index.md")),
    ("404.html", include_str!("../static/404.md")),
  ];
  sources
    .iter()
    .map(|(key, md)| (*key, render_md(md)))
    .collect()
});

#[cfg(not(feature = "hot-reload"))]
pub(crate) fn load_static(key: &str) -> Option<String> {
  RENDERED_PAGES.get(key).cloned()
}

// ---------------------------------------------------------------------------
// Hot-reload path — read from disk on every call.
// ---------------------------------------------------------------------------
#[cfg(feature = "hot-reload")]
pub(crate) fn load_static(key: &str) -> Option<String> {
  let (_, rel_path, fallback) = STATIC_FILE_MAPPING.iter().find(|(k, _, _)| *k == key)?;
  let md = crate::hot_reload::read(rel_path, fallback);
  Some(render_md(&md))
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Renders a Markdown string to a full HTML page using the site's stylesheet.
pub(crate) fn render_markdown_to_html(md: &str) -> String {
  render_md(md)
}


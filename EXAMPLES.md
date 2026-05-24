# API URL Examples

Root URL used in all examples: <http://localhost:8080>

## 1) Home page

URL:
<http://localhost:8080/>

What the URL components mean:
- `http://localhost:8080` = local server base URL
- `/` = root route

What you get back:
- `200 OK`
- HTML page rendered from `static/index.md`
- Content type is HTML

---

## 2) Favicon

URL:
<http://localhost:8080/favicon.ico>

What the URL components mean:
- `/favicon.ico` = favicon route

What you get back:
- `200 OK`
- favicon binary bytes
- `Content-Type: image/x-icon`

---

## 3) Install script for a supported app (default options)

URL:
<http://localhost:8080/v1/install/yq>

What the URL components mean:
- `/v1` = API namespace
- `/install` = install-script endpoints
- `/yq` = supported app shortname

What you get back:
- `200 OK` if matching release assets are found
- Script body (`.sh` by default)
- `Content-Type: application/x-sh`
- `Content-Disposition` includes an inline filename like `install-yq.sh`

---

## 4) Install script for a supported app (Linux + amd64 + specific version)

URL:
<http://localhost:8080/v1/install/jq?os=linux&arch=amd64&version=jq-1.7.1>

What the URL components mean:
- `/jq` = supported app
- `os=linux` = target operating system
- `arch=amd64` = target architecture
- `version=jq-1.7.1` = GitHub release tag to fetch (instead of `latest`)

What you get back:
- `200 OK` if assets exist for that app/version/target combo
- Bash install script for selected assets
- `Content-Type: application/x-sh`

---

## 5) Install script for Windows target

URL:
<http://localhost:8080/v1/install/gh?os=windows&arch=amd64>

What the URL components mean:
- `/gh` = supported app
- `os=windows` = selects PowerShell template path
- `arch=amd64` = target architecture

What you get back:
- `200 OK` if matching assets are found
- PowerShell script body
- `Content-Type: application/x-powershell`
- Inline filename like `install-gh.ps1`

---

## 6) View script inline in browser

URL:
<http://localhost:8080/v1/install/uv?inline=true>

What the URL components mean:
- `/uv` = supported app
- `inline=true` = return browser-friendly plain text response

What you get back:
- `200 OK`
- Script body rendered directly in the browser
- `Content-Type: text/plain; charset=utf-8`

---

## 7) Supported app with additional flags

URL:
<http://localhost:8080/v1/install/uv?prefix=/usr/local&force=true&quiet=true&log_level=INFO&method=binary&download_only=false>

What the URL components mean:
- `prefix=/usr/local` = installation prefix used by template logic
- `force=true` = force mode in script behavior
- `quiet=true` = reduced script output
- `log_level=INFO` = script log verbosity
- `method=binary` = install preference hint in template context
- `download_only=false` = normal install flow (not download-only mode)

What you get back:
- `200 OK` with install script (usually `.sh` unless `os=windows`)
- Script contains these options in rendered template context

---

## 8) Render highlighted HTML in browser via Accept header

URL:
<http://localhost:8080/v1/install/yq>

Example request header:
- `Accept: text/html`

What the URL components mean:
- `/yq` = supported app
- `Accept: text/html` header = return a full HTML document instead of raw script/plain text

What you get back:
- `200 OK`
- `Content-Type: text/html; charset=utf-8`
- Wide code block view with syntax highlighting
- Highlighted script body using shell/powershell language class

---

## 9) Arbitrary GitHub repository (latest release)

URL:
<http://localhost:8080/v1/install/cli/cli>

What the URL components mean:
- `/v1/install/{user}/{repo}` form
- `cli` (first segment) = GitHub org/user
- `cli` (second segment) = GitHub repository

What you get back:
- `200 OK` if matching assets exist for default target (`linux/amd64`)
- Install script for that repository
- Default script type is shell unless `os=windows`

---

## 10) Arbitrary GitHub repository with explicit target/version

URL:
<http://localhost:8080/v1/install/helm/helm?os=linux&arch=arm64&version=v3.16.1>

What the URL components mean:
- `/helm/helm` = fetch release assets from `github.com/helm/helm`
- `os=linux&arch=arm64` = target filter
- `version=v3.16.1` = use that exact tag

What you get back:
- `200 OK` and rendered install script if assets match
- Otherwise a JSON error response (for example no matching assets)

---

## 11) Unsupported supported-app route example

URL:
<http://localhost:8080/v1/install/not-a-real-app>

What the URL components mean:
- `/not-a-real-app` is not in the built-in supported app map

What you get back:
- `404 Not Found`
- JSON error body:
  - `error: "unsupported_app"`
  - `message: "Unsupported app: not-a-real-app"`

---

## 12) No matching assets example

URL:
<http://localhost:8080/v1/install/yq?os=netbsd&arch=mips64>

What the URL components mean:
- Valid app route (`yq`) but very restrictive target selection
- `os` and `arch` can produce an empty match set for release assets

What you get back:
- `404 Not Found` when nothing matches target/version filters
- JSON error body:
  - `error: "no_matching_assets"`
  - message describes repo and target

---

## Notes on query args

Common install query args:
- `os` (default: `linux`)
- `arch` (default: `amd64`)
- `version` (default: `latest`)
- `prefix` (default: `$HOME/.local`)
- `method` (`binary` or `installer`, default: `binary`)
- `download_only` (`true`/`false`, default: `false`)
- `force` (`true`/`false`, default: `false`)
- `quiet` (`true`/`false`, default: `false`)
- `log_level` (default: `DEBUG`)
- `inline` (`true`/`false`, default: `false`; when true, response is `text/plain` for browser viewing)
- `Accept: text/html` header (optional; when present, response is highlighted HTML)

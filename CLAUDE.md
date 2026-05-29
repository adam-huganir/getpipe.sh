# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Communication
<rules>
- Write for a global technical audience, including non-native English speakers.
- Use active voice and short sentences.
- Avoid idioms, metaphors, and filler words.
- Define technical terms in plain language on first use.
- Choose precise words; avoid words with multiple meanings.
</rules>
<example>
<bad>Endeavor to leverage your capabilities to synergize with the team.</bad>
<good>Use your skills to help the team.</good>
</example>

## Commands

Use `cargo xtask` for all common tasks. Use raw `cargo` only when you need options that xtask does not expose.

```bash
# Preferred: xtask wrappers
cargo xtask dev     # run dev server (LOG_LEVEL=debug, LOG_REQUESTS=true)
cargo xtask format  # run cargo fmt --all
cargo xtask lint    # run clippy --all-targets --all-features -D warnings
cargo xtask test    # run cargo test --all
cargo xtask build   # release build
cargo xtask clean   # clean build artifacts
cargo xtask ci      # format → lint → test → build (full pipeline)

# Raw cargo — use when xtask has no equivalent
cargo run -- serve --port 8080              # server with custom port
cargo run -- script install yq --os linux --arch amd64   # CLI mode
cargo run -- script install cli cli --os windows --arch amd64  # arbitrary GitHub repo
cargo test --workspace <test_name>          # single test by name
cargo test --workspace -- --nocapture       # show stdout during tests
cargo fmt --check                           # check formatting without writing (CI mode)
```

## Environment / Configuration

- Config is loaded from `config.yaml` at startup; falls back to defaults if missing. See `config.example.yaml`.
- Key env vars override config: `PORT`, `LISTEN_IP`, `LOG_LEVEL`, `LOG_REQUESTS`, `CORS_ALLOWED_ORIGINS`.
- `GETPIPE_ROOT` controls where static assets and templates are resolved from (default `../`).
- GitHub token: set via standard `GITHUB_TOKEN` / octocrab env var for higher API rate limits.

## Architecture

The binary is both an **HTTP server** and a **CLI tool** — `main()` dispatches on `cli::Commands`.

**Request flow (HTTP):**
1. Axum router in `main.rs` (`build_app`) routes `/v1/install/{app}` or `/v1/install/{user}/{repo}` to handlers.
2. Handlers call `services/installer.rs` — either `build_supported_install_script` (known apps) or `build_arbitrary_github_install_script` (arbitrary GitHub repos, with path-segment validation).
3. Installer calls `providers/gh.rs` (`get_github_download_links`) which hits the GitHub Releases API via octocrab. Results are cached with moka (TTL from config).
4. Links are passed to `services/templating.rs` which renders the appropriate shell template (`templates/install.sh` via Tera).
5. Response is wrapped in `http/responses.rs::ScriptResponse` — content type is `application/x-sh` or `application/x-powershell`; `?inline=true` or an HTML `Accept` header switches to plain text for browser viewing.

**Key modules:**
- `domain/platform.rs` — `TargetOs`, `TargetArch`, `TargetDeployment` enums; handles OS/arch normalization (amd64/x86_64 → single variant, arm64/aarch64 → single variant).
- `supported_apps.rs` — hardcoded map of ~10 known apps (yq, jq, gh, kubectl, helm, uv, etc.) with their GitHub repo + download path info.
- `config.rs` — YAML config loaded as a `LazyLock<Config>` static.
- `templates.rs` — Tera template registry, also a `LazyLock` static (`TEMPLATES`).
- `static_site.rs` — serves `static/` HTML/CSS files resolved relative to `GETPIPE_ROOT`.
- `error.rs` — `AppError` enum; implements `IntoResponse` for JSON error bodies with appropriate HTTP status codes.
- `http/query.rs` — `InstallQueryOptions` (query params: os, arch, version, prefix, inline, links_only).

**CLI flow** mirrors HTTP: `cli::ScriptInstallArgs::run()` calls the same `services/installer` functions.

## Routing

- `/` → static `index.html`
- `/install/{*rest}` → 307 redirect to `/v1/install/{*rest}` (keeps query string)
- `/v1/install/{app}` → supported app script
- `/v1/install/{user}/{repo}` → arbitrary GitHub repo script
- `/swagger-ui` + `/openapi.json` → utoipa-generated OpenAPI docs
- Fallback → static `404.html`

## Docker / Deployment

- `Dockerfile`: multi-stage, compiles to `x86_64-unknown-linux-musl` → minimal scratch image.
- CI (`publish.yaml`) builds and pushes to Google Artifact Registry, then deploys to Cloud Run on every push to `main`.
- `docker-bake.json` supports multi-platform builds.

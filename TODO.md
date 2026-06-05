# TODO

Features that were stubbed or partially built. Removed from the codebase to avoid dead code.

---

## Native installer (`getpipe install`)

**What was there:** A `Commands::Install` subcommand backed by `InstallArgs` and
`NativeInstallPlan`. The flow was:

1. Resolve GitHub download links for a target app or arbitrary `owner/repo`.
2. Present an interactive menu to pick from matching assets.
3. Download the selected asset to a temp directory.
4. For bare binaries: copy to a user-chosen destination.
5. For `.tar.gz` / `.tar` archives: list entries, let the user pick one, extract it,
   copy to destination.
6. Return success.

**What was missing:** Step 6. After `finalize_install` copied the file and logged
"Copied to {path}", the function fell through to an unconditional
`Err(AppError::InvalidInput("Native install not yet implemented ..."))`.
There was no `return Ok(...)` and no `CliInstallOutput::Installed` variant to carry
the result. Everything else -- download, extraction, destination prompt, overwrite
guard -- was written and reachable.

**To implement:**

- Add `CliInstallOutput::Installed { path: PathBuf }` (or similar).
- Return `Ok(CliInstallOutput::Installed { path: final_path })` after a successful copy.
- Print a confirmation line (ideally colored via `crossterm`).
- Wire `Commands::Install` back into `main.rs`.
- Re-add crate deps: `crossterm`, `flate2`, `tar`.

**Missing archive formats:** The extraction code handled `.tar` and `.tar.gz`/`.tgz`
only. `.tar.xz` was detected by `is_archive` but not extractable. `.zip` was not
handled at all. The PowerShell template (`templates/install.ps1`) handles both via
`tar` / `7z` -- the Rust side needs the same.

**Missing post-install steps:**
- Set executable bit on Unix (`std::fs::set_permissions` + `std::os::unix::fs::PermissionsExt`).
- Optionally offer to add the destination directory to `PATH` (write to `~/.bashrc` /
  `~/.zshrc` / Windows user `PATH` env var).

**Environment variable overrides:** `NativeInstallPlan::from_args` read
`GETPIPE_OS`, `GETPIPE_ARCH`, `GETPIPE_VERSION`, `GETPIPE_PREFIX`,
`GETPIPE_METHOD`, `GETPIPE_DOWNLOAD_ONLY`, `GETPIPE_FORCE`, `GETPIPE_QUIET`,
`GETPIPE_LOG_LEVEL` and merged them with CLI flags (CLI wins). This behavior is
worth preserving when the command is restored.

---

## PowerShell install template (`templates/install.ps1`)

**What is there:** A complete PowerShell 3+ script template that handles binary
downloads, `.tar.gz` extraction, MSI/EXE installer launch, and interactive asset
selection. It is rendered by `services/templating.rs` for `TargetOs::Windows` and
served via the HTTP API.

**What is missing:** The template is tested only by a stub in
`src/templates.rs::tests` (writes `# stub\n` to a temp file). There are no
integration tests that render the template with real asset data and verify the
output. Add tests equivalent to those in `http/responses.rs` for the bash template.

---

## Supported apps list

**What is there:** ~10 hardcoded apps in `src/supported_apps.rs`.

**What is missing:** No way to add apps without recompiling. A YAML/TOML config file
overlay (merged at startup alongside `config.yaml`) would let operators add apps
without a code change. The `config.example.yaml` could document the schema.

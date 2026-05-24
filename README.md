# termlibs-server

A simple server for the termlibs project to build and serve scripts to be run by the caller.

## Usage

an example usage from the command line:
```bash
curl https://termlibs.dev/install/yq?version=4.44.3 | bash -s -- --prefix $HOME/.local
```

## CLI usage

The CLI mirrors the `/v1/install` API and now separates script generation from (future) native installs.

### Scripts (templated)
```bash
# supported app
termlibs script install yq --os linux --arch amd64 --version 4.44.3 --prefix $HOME/.local

# arbitrary GitHub repo
termlibs script install cli cli --os windows --arch amd64

# links-only output (filename -> url)
termlibs script install yq --links-only

# shell completions
termlibs completions bash > /etc/bash_completion.d/termlibs
```

### Native install (placeholder)
```bash
termlibs install yq
# currently returns a TODO message; use `termlibs script install` for scripts
```

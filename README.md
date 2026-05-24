# getpipe.sh-server

A simple server for the getpipe.sh project to build and serve scripts to be run by the caller.

## Usage

an example usage from the command line:
```bash
curl https://getpipe.sh/install/yq?version=4.44.3 | bash -s -- --prefix $HOME/.local
```

## CLI usage

The CLI mirrors the `/v1/install` API and now separates script generation from (future) native installs.

### Scripts (templated)
```bash
# supported app
getpipe.sh script install yq --os linux --arch amd64 --version 4.44.3 --prefix $HOME/.local

# arbitrary GitHub repo
getpipe.sh script install cli cli --os windows --arch amd64

# links-only output (filename -> url)
getpipe.sh script install yq --links-only

# shell completions
getpipe.sh completions bash > /etc/bash_completion.d/getpipe.sh
```

### Native install (placeholder)
```bash
getpipe.sh install yq
# currently returns a TODO message; use `getpipe.sh script install` for scripts
```

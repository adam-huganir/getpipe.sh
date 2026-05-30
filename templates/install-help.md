# getpipe.sh — Install Help

Generate and pipe shell install scripts for popular CLI tools.

---

## Examples

### Supported app — default options

```bash
curl https://getpipe.sh/v1/install/yq | bash
```

### Supported app — custom options

```bash
curl "https://getpipe.sh/v1/install/yq?os=linux&arch=amd64&version=4.44.3&prefix=/usr/local" | bash
```

### Arbitrary GitHub release

```bash
curl https://getpipe.sh/v1/install/mikefarah/yq | bash
```

### Windows (PowerShell)

```powershell
irm https://getpipe.sh/v1/install/yq | iex
```

---

## Query Parameters

| Parameter | Values | Default |
|-----------|--------|---------|
| `os` | `linux`, `mac`, `windows`, `freebsd`, `openbsd`, `netbsd` | `linux` |
| `arch` | `amd64`, `arm64`, `arm`, `x86`, `ppc64le`, `ppc64`, `mips64le`, `mips64`, `mipsle`, `mips`, `riscv` | `amd64` |
| `version` | any release tag | `latest` |
| `prefix` | install directory path | auto-detected per OS |

> [!NOTE]
> **`prefix` auto-detection**
>
> **bash (Linux / macOS / BSD)** — checked in order:
> 1. Running as `root` → `/usr/local`
> 2. `$HOME/.local/bin` exists → `$HOME/.local`
> 3. `$HOME/bin` exists → `$HOME`
> 4. Fallback: the directory the script was invoked from
>
> **PowerShell (Windows)** — checked in order:
> 1. Running as Administrator → `%ProgramFiles%`
> 2. `%LOCALAPPDATA%\Programs` exists → `%LOCALAPPDATA%\Programs`
> 3. Fallback: the directory the script was invoked from

---

## Supported Apps

| App | Repository |
|-----|------------|
{% for app in apps -%}
| `{{ app.name }}` | [{{ app.repo }}]({{ app.github_url }}) |
{% endfor %}

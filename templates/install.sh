#!/usr/bin/env bash
{# template engine Tera #}
{%- if (assets | length  > 0) %}
#------------------------------------------------------------------------------
# 01) Matching Artifacts
#------------------------------------------------------------------------------
_artifacts=( {% for asset in assets %}
  {{ asset.url | escape_shell }}
{%- endfor %}
)

#------------------------------------------------------------------------------
# 02) Runtime Setup
#------------------------------------------------------------------------------
set -euo pipefail
_E_GENERIC_ERROR=1
_ORIG_DIR="$PWD"
_FORCE={{ force | escape_shell }}
_CANONICAL_BINARY_NAME={{ app | escape_shell }}

#------------------------------------------------------------------------------
# 03) Temporary Workspace and Exit Cleanup
#------------------------------------------------------------------------------
_TMPDIR="$(mktemp -d)"
cd "$_TMPDIR"
trap "[ -d \"$_TMPDIR\" ] && printf 'Removing %s\n' \"$_TMPDIR\" >&2 && rm -rf \"$_TMPDIR\"" EXIT

#------------------------------------------------------------------------------
# 04) Interactive Choice Prompt
#------------------------------------------------------------------------------
_ask_choices() {
  local OPTIND opt add_none add_quit idx choices choice
  add_none=false
  add_quit=false
  while getopts "nq" opt; do
    case "$opt" in
      n) add_none=true ;;
      q) add_quit=true ;;
      *) printf "invalid option\n" >&2; exit 1 ;;
    esac
  done
  shift $((OPTIND - 1))
  choices=("$@")
  {% raw %}
  if [ "${#choices[@]}" -eq 0 ]; then
    printf "no choices provided\n" >&2
    exit 1
  fi
  {% endraw %}

  idx=1
  for c in "${choices[@]}"; do
    printf "\t%s)\t%s\n" "$idx" "$c" 1>&2
    idx=$((idx + 1))
  done

  if [ "$add_none" = true ]; then
    printf "\tn)\tnone\n" 1>&2
  fi
  if [ "$add_quit" = true ]; then
    printf "\tq)\tquit\n" 1>&2
  fi

  printf "Enter choice: " 1>&2
  read -r choice </dev/tty

  local final_choices=()
  for c in $choice; do
    case "$c" in
      [0-9]*)
        c=$((c - 1))
    esac
    final_choices+=("$c")
  done
  echo "${final_choices[@]}"
}


#------------------------------------------------------------------------------
# 05) Download Helper
#------------------------------------------------------------------------------
_urlget() {
  if command -v curl &> /dev/null; then
    curl -fsSL "$1" 2> /dev/null
  elif command -v wget &> /dev/null; then
    wget -qO- "$1" 2> /dev/null
  else
    printf "neither curl nor wget found, unable to download files\n" >&2
    return "$_E_GENERIC_ERROR"
  fi
}

#------------------------------------------------------------------------------
# 06) Overwrite Guard
#------------------------------------------------------------------------------
# Exits with 0 (skip) if the destination already exists and the user declines.
# Skipped entirely when _FORCE='true'.
_confirm_overwrite() {
  local dest="$1" _ow_answer
  if [ -e "$dest" ] && [ "$_FORCE" != 'true' ]; then
    printf "%s already exists. Overwrite? [y/N] " "$dest" >&2
    read -r _ow_answer </dev/tty
    case "$_ow_answer" in
      [yY]|[yY][eE][sS]) ;;
      *) printf "skipping installation\n" >&2; exit 0 ;;
    esac
  fi
}

#------------------------------------------------------------------------------
# 07) Installation Prefix
#------------------------------------------------------------------------------
{% if prefix and prefix != "auto" %}
RUN_DIRECTORY={{ prefix | escape_shell }}
{% else %}
_detect_prefix() {
  # 1. Running as root → system-wide location
  if [ "$(id -u)" = "0" ]; then
    printf "/usr/local"
    return
  fi
  # 2. Per-user local bin exists → use $HOME/.local
  if [ -d "$HOME/.local/bin" ]; then
    printf "%s/.local" "$HOME"
    return
  fi
  # 3. $HOME/bin exists → use $HOME
  if [ -d "$HOME/bin" ]; then
    printf "%s" "$HOME"
    return
  fi
  # 4. Fallback: directory from which the script was invoked
  printf "%s" "$_ORIG_DIR"
}
RUN_DIRECTORY="$(_detect_prefix)"
{% endif %}

#------------------------------------------------------------------------------
# 08) Asset Arrays
#------------------------------------------------------------------------------
_filenames=( {% for asset in assets %}{{ asset.name | escape_shell }} {% endfor %})
_filetypes=( {% for asset in assets %}{{ asset.filetype | escape_shell }} {% endfor %})
_printables=( {% for asset in assets %}{{ asset.name ~ " (" ~ asset.filetype ~ ")" | escape_shell }} {% endfor %})

#------------------------------------------------------------------------------
# 09) Asset Selection
#------------------------------------------------------------------------------
printf "Please select one of the following:\n"
choice="$(_ask_choices -q "${_printables[@]}")"

#------------------------------------------------------------------------------
# 10) Selection Validation
#------------------------------------------------------------------------------
case "$choice" in
  q|n)
    exit 0
    ;;
  [0-9]*)
    if ! [ "$choice" -lt {% raw %}"${#_artifacts[@]}"{% endraw %} ]; then
      printf "invalid choice: %s\n" "$choice" >&2
      exit 100
    fi
    ;;
  *)
    printf "invalid choice: %s\n" "$choice" >&2
    exit 100
    ;;
esac

#------------------------------------------------------------------------------
# 11) Download and Install Dispatch
#------------------------------------------------------------------------------
printf "Downloading from %s to %s\n" "${_artifacts[$choice]}" "$_TMPDIR"
_type="${_filetypes[$choice]}"
case "$_type" in
  "binary" | "deb installer")
    filename="${_filenames[$choice]}"
    saved_file="$_TMPDIR/$filename"
    _urlget "${_artifacts[$choice]}" > "$saved_file"

    if [ "$_type" = "deb installer" ]; then
      if command -v dpkg &> /dev/null; then
        printf "trying to install with dpkg, this may prompt for sudo\n"
        dpkg -i "$saved_file" || sudo dpkg -i "$saved_file"
      else
        printf "dpkg not found, unable to install package\n" >&2
        exit 100
      fi
    elif [ "$_type" = "binary" ]; then
      chmod +x "$saved_file"

      if [ -z "$_CANONICAL_BINARY_NAME" ]; then
        read -r -p "enter alternate binary name (default: $filename): " binary_name </dev/tty
        binary_name="${binary_name:-$filename}"
      else
        binary_name="$_CANONICAL_BINARY_NAME"
      fi
      read -r -p "enter alternate binary directory (default: $RUN_DIRECTORY/bin): " binary_dir </dev/tty
      binary_dir="${binary_dir:-$RUN_DIRECTORY/bin}"
      mkdir -p "$binary_dir"
      _confirm_overwrite "$binary_dir/$binary_name"
      cp "$saved_file" "$binary_dir/$binary_name"
    else
      printf "invalid filetype: %s\n" "$_type" >&2
      exit 100
    fi
    ;;
  "tar.gz")
    _urlget "${_artifacts[$choice]}" | tar xz
    executable_files=(
      $(find . -type f -executable -exec printf '{} ' \;)
    )
    {# raw block here to allow for the comment looking shell op #}
    {% raw %}
    if [ "${#executable_files[@]}" -eq 0 ]; then
    {% endraw %}
      printf "no executable files found in archive\n" >&2
      exit 100
    else
      choices="$(_ask_choices -q "${executable_files[@]}")"
    fi
    for choice in $choices; do
      case "$choice" in
        [0-9]*)
          mkdir -p "$RUN_DIRECTORY/bin"
          if [ -n "$_CANONICAL_BINARY_NAME" ]; then
            _dest_name="$_CANONICAL_BINARY_NAME"
          else
            _dest_name="$(basename "${executable_files[$choice]}")"
          fi
          _confirm_overwrite "$RUN_DIRECTORY/bin/$_dest_name"
          cp "${executable_files[$choice]}" "$RUN_DIRECTORY/bin/$_dest_name"
          ;;
      esac
    done
    ;;
  *)
    printf "invalid filetype: %s\n" "${_filetypes[$choice]}" >&2
    exit 100
    ;;
esac
{% else %}
#------------------------------------------------------------------------------
# 1) No Assets Available
#------------------------------------------------------------------------------
printf "no assets found\n" >&2
exit 100
{% endif %}

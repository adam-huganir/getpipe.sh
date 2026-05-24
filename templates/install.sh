#!/usr/bin/env bash

#{# template engine Tera #}

#------------------------------------------------------------------------------
# 01) Runtime Setup
#------------------------------------------------------------------------------
set -euo pipefail
{% if (assets | length  > 0) %}
RUN_DIRECTORY="$PWD"
_QUIET={{ quiet | escape_shell }}
_FORCE={{ force | escape_shell }}
_CANONICAL_BINARY_NAME={{ app | escape_shell }}

_E_GENERIC_ERROR=1

#------------------------------------------------------------------------------
# 02) Temporary Workspace and Exit Cleanup
#------------------------------------------------------------------------------
_TMPDIR="$(mktemp -d)"
cd "$_TMPDIR"
trap "[ -d \"$_TMPDIR\" ] && printf 'Removing %s\n' \"$_TMPDIR\" >&2 && rm -rf \"$_TMPDIR\"" EXIT

#------------------------------------------------------------------------------
# 03) Interactive Choice Prompt
#------------------------------------------------------------------------------
_ask_choices() {
  local choice choices opt add_none add_quit idx
  opt="$(getopt -o "" --long "none,quit" -n "${FUNCNAME[0]}" -- "$@")"
  if [ "$?" -ne 0 ]; then
    printf "invalid options\n" >&2
    exit 1
  fi
  eval set -- "$opt"
  add_none=false
  add_quit=false
  while true; do
    case "$1" in
      --quit)
        add_quit=true
        shift
        ;;
      --none)
        add_none=true
        shift
        ;;
      --)
        shift
        break
        ;;
    esac
  done
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
# 04) Download Helper
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
# 05) Rendered Asset Arrays
#------------------------------------------------------------------------------
_urls=( {% for asset in assets %}
  {{ asset.url | escape_shell }}
{%- endfor %}
)
_filenames=( {% for asset in assets %}{{ asset.name | escape_shell }} {% endfor %})
_filetypes=( {% for asset in assets %}{{ asset.filetype | escape_shell }} {% endfor %})
_printables=( {% for asset in assets %}{{ asset.name ~ " (" ~ asset.filetype ~ ")" | escape_shell }} {% endfor %})

#------------------------------------------------------------------------------
# 06) Asset Selection
#------------------------------------------------------------------------------
printf "Please select one of the following:\n"
choice="$(_ask_choices --quit "${_printables[@]}")"

#------------------------------------------------------------------------------
# 07) Selection Validation
#------------------------------------------------------------------------------
case "$choice" in
  q|n)
    exit 0
    ;;
  [0-9]*)
    if ! [ "$choice" -lt {% raw %}"${#_urls[@]}"{% endraw %} ]; then
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
# 08) Download and Install Dispatch
#------------------------------------------------------------------------------
printf "Downloading from %s to %s\n" "${_urls[$choice]}" "$_TMPDIR"
_type="${_filetypes[$choice]}"
case "$_type" in
  "binary" | "deb installer")
    filename="${_filenames[$choice]}"
    saved_file="$_TMPDIR/$filename"
    _urlget "${_urls[$choice]}" > "$saved_file"

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
      cp "$saved_file" "$binary_dir/$binary_name"
    else
      printf "invalid filetype: %s\n" "$_type" >&2
      exit 100
    fi
    ;;
  "tar.gz")
    filename="${_filenames[$choice]}"
    _urlget "${_urls[$choice]}" | tar xz
    executable_files=(
      $(find . -type f -executable -exec printf '{} ' \;)
    )
    {% raw %}
    if [ "${#executable_files[@]}" -eq 0 ]; then  {# raw block here to allow for the comment looking shell op #}
    {% endraw %}
      printf "no executable files found in archive\n" >&2
      exit 100
    else
      choices="$(_ask_choices --quit "${executable_files[@]}")"
    fi
    for choice in $choices; do
      case "$choice" in
        [0-9]*)
          cp "${executable_files[$choice]}" "$RUN_DIRECTORY/bin"
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
# 09) No Assets Available
#------------------------------------------------------------------------------
printf "no assets found\n" >&2
exit 100
{% endif %}

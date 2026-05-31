#!/usr/bin/env bash

#------------------------------------------------------------------------------
# 01) Matching Artifacts
#------------------------------------------------------------------------------
_artifacts=(
  $'https://github.com/mikefarah/yq/releases/download/v4.53.2/yq_linux_amd64'
  $'https://github.com/mikefarah/yq/releases/download/v4.53.2/yq_linux_amd64.tar.gz'
)

#------------------------------------------------------------------------------
# 04) Interactive Choice Prompt
#------------------------------------------------------------------------------
_ask_choices() {
  local OPTIND opt add_none add_quit idx choices choice c
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

  if [ "${#choices[@]}" -eq 0 ]; then
    printf "no choices provided\n" >&2
    exit 1
  fi

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
  while IFS= read -r c; do
    [ -z "$c" ] && continue
    case "$c" in
      [0-9]*)
        c=$((c - 1))
    esac
    final_choices+=("$c")
  done < <(printf '%s\n' "$choice" | tr ' ' '\n')
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
    return 1
  fi
}

#------------------------------------------------------------------------------
# 06) Overwrite Guard
#------------------------------------------------------------------------------
_confirm_overwrite() {
  local dest="$1" _ow_answer
  if [ -e "$dest" ] && [ "false" != 'true' ]; then
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
_detect_prefix() {
  if [ "$(id -u)" = "0" ]; then
    printf "/usr/local"
    return
  fi
  if [ -d "$HOME/.local/bin" ]; then
    printf "%s/.local" "$HOME"
    return
  fi
  if [ -d "$HOME/bin" ]; then
    printf "%s" "$HOME"
    return
  fi
  printf "%s" "$_ORIG_DIR"
}

#------------------------------------------------------------------------------
# Main
#------------------------------------------------------------------------------
_pipe_install_yq() {
  if [ -n "$ZSH_VERSION" ]; then emulate -L bash; fi
  (
    set -euo pipefail

    #------------------------------------------------------------------------------
    # 03) Temporary Workspace and Exit Cleanup
    #------------------------------------------------------------------------------
    _ORIG_DIR="$(pwd)"
    _TMPDIR="$(mktemp -d)"
    cd "$_TMPDIR"
    trap "[ -d \"$_TMPDIR\" ] && printf 'Removing %s\n' \"$_TMPDIR\" >&2 && rm -rf \"$_TMPDIR\"" EXIT

    #------------------------------------------------------------------------------
    # 07) Installation Prefix (continued)
    #------------------------------------------------------------------------------
    RUN_DIRECTORY="$(_detect_prefix)"

    #------------------------------------------------------------------------------
    # 08) Asset Arrays
    #------------------------------------------------------------------------------
    _filenames=( yq_linux_amd64 yq_linux_amd64.tar.gz )
    _filetypes=( binary tar.gz )
    _printables=( $'yq_linux_amd64 (binary)' $'yq_linux_amd64.tar.gz (tar.gz)' )

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
        if ! [ "$choice" -lt "${#_artifacts[@]}" ]; then
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

          if [ -z "yq" ]; then
            read -r -p "enter alternate binary name (default: $filename): " binary_name </dev/tty
            binary_name="${binary_name:-$filename}"
          else
            binary_name="yq"
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
        executable_files=()
        while IFS= read -r -d '' f; do
          executable_files+=("$f")
        done < <(find . -type f -executable -print0)

        if [ "${#executable_files[@]}" -eq 0 ]; then
          printf "no executable files found in archive\n" >&2
          exit 100
        else
          choices="$(_ask_choices -q "${executable_files[@]}")"
        fi
        for choice in $choices; do
          case "$choice" in
            [0-9]*)
              mkdir -p "$RUN_DIRECTORY/bin"
              if [ -n "yq" ]; then
                _dest_name="yq"
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
  )
}

if [[ -n "${ZSH_VERSION-}" ]]; then
  [[ "${(%):-%x}" == "${0}" ]] && _pipe_install_yq
elif [[ "${BASH_SOURCE[0]-}" == "${0}" ]]; then
  _pipe_install_yq
fi
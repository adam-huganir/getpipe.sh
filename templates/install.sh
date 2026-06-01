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

{% raw %}
#------------------------------------------------------------------------------
# 04) Terminal Cursor Primitives
#------------------------------------------------------------------------------
_cur_up()      { printf '\033[%dA' "${1:-1}" >&2; }
_cur_down()    { printf '\033[%dB' "${1:-1}" >&2; }
_cur_forward() { printf '\033[%dC' "${1:-1}" >&2; }
_cur_back()    { printf '\033[%dD' "${1:-1}" >&2; }
_cur_bol()     { printf '\033[1G'  >&2; }
_cur_save()    { printf '\033[s'   >&2; }
_cur_restore() { printf '\033[u'   >&2; }
_cur_hide()    { printf '\033[?25l' >&2; }
_cur_show()    { printf '\033[?25h' >&2; }
_clear_line()  { printf '\033[2K'  >&2; }
_line_bold()    { printf '\033[1m'  >&2; }
_line_normal()  { printf '\033[0m'  >&2; }
_line_reverse() { printf '\033[7m'  >&2; }

#------------------------------------------------------------------------------
# 05) Multi-Select Menu
#------------------------------------------------------------------------------
_draw_menu() {
  local single="$1" current="$2"; shift 2
  local items=("$@")
  local i sel width=${#items[@]}; width=${#width}
  for i in "${!items[@]}"; do
    sel="${_MENU_SELECTED[$i]:-0}"
    _cur_bol; _clear_line
    printf '  ' >&2
    if [ "$i" -eq "$current" ]; then
      _line_reverse; printf '%*d)' "$width" "$((i + 1))" >&2; _line_normal
    else
      printf '%*d)' "$width" "$((i + 1))" >&2
    fi
    printf ' ' >&2
    if [ "$sel" = "1" ]; then
      _line_reverse; printf '%s' "${items[$i]}" >&2; _line_normal
    else
      printf '%s' "${items[$i]}" >&2
    fi
    printf '\n' >&2
  done
  local n="${#items[@]}" a_idx q_idx
  if $single; then
    q_idx=$n
  else
    a_idx=$n; q_idx=$((n + 1))
    _cur_bol; _clear_line; printf '  ' >&2
    if [ "$current" -eq "$a_idx" ]; then
      _line_reverse; printf 'a)' >&2; _line_normal
    else
      printf 'a)' >&2
    fi
    printf ' all\n' >&2
  fi
  _cur_bol; _clear_line; printf '  ' >&2
  if [ "$current" -eq "$q_idx" ]; then
    _line_reverse; printf 'q)' >&2; _line_normal
  else
    printf 'q)' >&2
  fi
  printf ' quit\n' >&2
}

_multi_select() {
  local single=false
  [[ "${1-}" == "-1" ]] && { single=true; shift; }
  local items=("$@")
  local n="${#items[@]}"
  local extra; $single && extra=1 || extra=2
  local current=0 idx i key rest
  _MENU_SELECTED=()
  for ((i=0; i<n; i++)); do _MENU_SELECTED+=("0"); done

  _cur_hide
  _draw_menu "$single" "$current" "${items[@]}"

  while true; do
    IFS= read -rsn1 key </dev/tty
    if [[ "$key" == $'\033' ]]; then
      read -rsn2 -t 0.1 rest </dev/tty
      key="$key$rest"
    fi
    case "$key" in
      $'\033[A') # up
        [ "$current" -gt 0 ] && current=$((current - 1))
        ;;
      $'\033[B') # down
        [ "$current" -lt $((n + extra - 1)) ] && current=$((current + 1))
        ;;
      [1-9])
        idx=$((key - 1))
        [ "$idx" -lt "$n" ] && current=$idx
        ;;
      ' ')
        local on_a=false on_q=false
        ! $single && [ "$current" -eq "$n" ]          && on_a=true
        $single    && [ "$current" -eq "$n" ]          && on_q=true
        ! $single  && [ "$current" -eq $((n + 1)) ]   && on_q=true
        if $on_q; then
          _cur_show; return 1
        elif $on_a; then
          local all_on=true
          for ((i=0; i<n; i++)); do
            [ "${_MENU_SELECTED[$i]}" != "1" ] && { all_on=false; break; }
          done
          local v; $all_on && v="0" || v="1"
          for ((i=0; i<n; i++)); do _MENU_SELECTED[$i]="$v"; done
        elif $single; then
          _MENU_SELECTED[$current]="1"
          _cur_up $((n + extra))
          _draw_menu "$single" "$current" "${items[@]}"
          _cur_show; printf '%d\n' "$current"; return 0
        else
          if [ "${_MENU_SELECTED[$current]}" = "1" ]; then
            _MENU_SELECTED[$current]="0"
          else
            _MENU_SELECTED[$current]="1"
          fi
        fi
        ;;
      '')
        local on_a=false on_q=false
        ! $single && [ "$current" -eq "$n" ]          && on_a=true
        $single    && [ "$current" -eq "$n" ]          && on_q=true
        ! $single  && [ "$current" -eq $((n + 1)) ]   && on_q=true
        if $on_q; then
          _cur_show; return 1
        fi
        if $single; then
          _MENU_SELECTED[$current]="1"
        elif $on_a; then
          for ((i=0; i<n; i++)); do _MENU_SELECTED[$i]="1"; done
        else
          local any_selected=false
          for i in "${!_MENU_SELECTED[@]}"; do
            [ "${_MENU_SELECTED[$i]}" = "1" ] && { any_selected=true; break; }
          done
          if ! $any_selected && [ "$current" -lt "$n" ]; then
            _MENU_SELECTED[$current]="1"
          fi
        fi
        _cur_up $((n + extra))
        _draw_menu "$single" "$current" "${items[@]}"
        _cur_show
        if $single; then
          printf '%d\n' "$current"
        else
          for i in "${!_MENU_SELECTED[@]}"; do
            [ "${_MENU_SELECTED[$i]}" = "1" ] && printf '%d ' "$i"
          done
          printf '\n'
        fi
        return 0
        ;;
      a|A)
        if ! $single; then
          local all_on=true
          for ((i=0; i<n; i++)); do
            [ "${_MENU_SELECTED[$i]}" != "1" ] && { all_on=false; break; }
          done
          local v; $all_on && v="0" || v="1"
          for ((i=0; i<n; i++)); do _MENU_SELECTED[$i]="$v"; done
        fi
        ;;
      q|Q)
        _cur_show; return 1
        ;;
    esac
    _cur_up $((n + extra))
    _draw_menu "$single" "$current" "${items[@]}"
  done
}
{% endraw %}

#------------------------------------------------------------------------------
# Logging
#------------------------------------------------------------------------------
_debug() {
  [ "${GETPIPE_LOG_LEVEL:-}" = "DEBUG" ] && printf '[debug] %s\n' "$*" >&2
  return 0
}

#------------------------------------------------------------------------------
# 06) Download Helper
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
# 07) Overwrite Guard
#------------------------------------------------------------------------------
# Exits with 0 (skip) if the destination already exists and the user declines.
# Skipped entirely when _FORCE='true'.
_confirm_overwrite() {
  local dest="$1" _ow_answer
  if [ -e "$dest" ] && [ "{{ force | escape_shell }}" != 'true' ]; then
    printf "%s already exists. Overwrite? [y/N] " "$dest" >&2
    read -r _ow_answer </dev/tty
    case "$_ow_answer" in
      [yY]|[yY][eE][sS]) ;;
      *) printf "skipping installation\n" >&2; exit 0 ;;
    esac
  fi
}

#------------------------------------------------------------------------------
# 08) Installation Prefix
#------------------------------------------------------------------------------
{% if prefix and prefix != "auto" %}
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
{% endif %}

#------------------------------------------------------------------------------
# Main
#------------------------------------------------------------------------------
_pipe_install() {
  if [ -n "$ZSH_VERSION" ]; then emulate -L bash; fi
  (
    set -euo pipefail

    #--------------------------------------------------------------------------
    # 03) Temporary Workspace and Exit Cleanup
    #--------------------------------------------------------------------------
    _ORIG_DIR="$(pwd)"
    _TMPDIR="$(mktemp -d)"
    _debug "workdir: $_TMPDIR"
    cd "$_TMPDIR"
    trap "[ -d \"$_TMPDIR\" ] && printf 'Removing %s\n' \"$_TMPDIR\" >&2 && rm -rf \"$_TMPDIR\"" EXIT

    #--------------------------------------------------------------------------
    # 08) Installation Prefix (continued)
    #--------------------------------------------------------------------------
    {% if prefix and prefix != "auto" %}
    RUN_DIRECTORY={{ prefix | escape_shell }}
    {% else %}
    RUN_DIRECTORY="$(_detect_prefix)"
    {% endif %}
    _debug "prefix: $RUN_DIRECTORY"

    #--------------------------------------------------------------------------
    # 09) Asset Arrays
    #--------------------------------------------------------------------------
    _filenames=( {% for asset in assets %}{{ asset.name | escape_shell }} {% endfor %})
    _filetypes=( {% for asset in assets %}{{ asset.filetype | escape_shell }} {% endfor %})
    _printables=( {% for asset in assets %}{{ asset.name ~ " (" ~ asset.filetype ~ ")" | escape_shell }} {% endfor %})

    #--------------------------------------------------------------------------
    # 10) Asset Selection
    #--------------------------------------------------------------------------
    printf "Please select one of the following:\n" >&2
    choice="$(_multi_select -1 "${_printables[@]}")" || exit 0
    _debug "selected: $choice"

    #--------------------------------------------------------------------------
    # 11) Download and Install Dispatch
    #--------------------------------------------------------------------------
    printf "Downloading from %s to %s\n" "${_artifacts[$choice]}" "$_TMPDIR"
    _type="${_filetypes[$choice]}"
    _debug "artifact: ${_artifacts[$choice]}, type: $_type"
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

          if [ -n "{{ app | escape_shell }}" ]; then
            _default_name="$(basename "{{ app | escape_shell }}")"
          else
            _default_name="$filename"
          fi
          read -r -e -i "$_default_name" -p "Binary name: " binary_name </dev/tty
          binary_name="${binary_name:-$_default_name}"
          read -r -e -i "$RUN_DIRECTORY/bin" -p "Install directory: " binary_dir </dev/tty
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
          _debug "found: $f"
        done < <(find . -type f -executable -print0)

        {# raw block here to allow for the comment looking shell op #}
        {% raw %}
        if [ "${#executable_files[@]}" -eq 0 ]; then
        {% endraw %}
          printf "no executable files found in archive\n" >&2
          exit 100
        {% raw %}
        elif [ "${#executable_files[@]}" -eq 1 ]; then
        {% endraw %}
          choices="0"
        else
          printf "Select binaries to install:\n" >&2
          choices="$(_multi_select "${executable_files[@]}")" || exit 0
        fi

        read -r -e -i "$RUN_DIRECTORY/bin" -p "Install directory: " install_dir </dev/tty
        install_dir="${install_dir:-$RUN_DIRECTORY/bin}"
        mkdir -p "$install_dir"

        to_install=()
        _dest_names=()
        # TODO: currently this will use the app default name
        #       multiple times if the user doesn't change it
        #       we should probably do a explicit fuzzy map
        for choice in $choices; do
          _src_name="$(basename "${executable_files[$choice]}")"
          if [ -n "{{ app | escape_shell }}" ]; then
            _default_name="$(basename "{{ app | escape_shell }}")"
          else
            _default_name="$_src_name"
          fi
          read -r -e -i "$_default_name" -p "Install '$_src_name' as: " _dest_name </dev/tty
          _dest_name="${_dest_name:-$_default_name}"
          _dest_path="$install_dir/$_dest_name"
          if [ -e "$_dest_path" ]; then
            read -r -p "$_dest_path already exists. Overwrite? [y/N] " _ow </dev/tty
            case "$_ow" in
              [yY]|[yY][eE][sS]) to_install+=("$choice"); _dest_names+=("$_dest_name") ;;
              *) printf "skipping %s\n" "$_dest_name" >&2 ;;
            esac
          else
            to_install+=("$choice")
            _dest_names+=("$_dest_name")
          fi
        done

        for i in "${!to_install[@]}"; do
          choice="${to_install[$i]}"
          _dest_name="${_dest_names[$i]}"
          printf "installing: ${executable_files[$choice]} -> $install_dir/$_dest_name\n" >&2
          cp "${executable_files[$choice]}" "$install_dir/$_dest_name"
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
  [[ "${(%):-%x}" == "${0}" || -z "${(%):-%x}" ]] && _pipe_install
elif [[ -z "${BASH_SOURCE[0]-}" || "${BASH_SOURCE[0]-}" == "${0}" ]]; then
  _pipe_install
fi
{% else %}
#------------------------------------------------------------------------------
# 1) No Assets Available
#------------------------------------------------------------------------------
printf "no assets found\n" >&2
exit 100
{% endif %}

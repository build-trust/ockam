#!/bin/bash

set -e

# This script fetches precompiled released Ockam binaries and
# stores them in the current directory.
# https://github.com/build-trust/ockam/releases

# You can call it as follow:
#
# To install the latest released version:
# curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/build-trust/ockam/develop/install.sh | bash
#
# Or
#
# To install a specific version:
# curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/build-trust/ockam/develop/install.sh | bash -s -- --version v0.80.0

# It borrows ideas from the MIT Licensed rustup-init script which has
# been used and tested in many environments over many years
# https://github.com/rust-lang/rustup/blob/master/rustup-init.sh

ansi_escapes_are_valid() {
  local _ansi_escapes_are_valid=false
  if [ -t 2 ]; then
    if [ "${TERM+set}" = 'set' ]; then
      case "$TERM" in
      xterm* | rxvt* | urxvt* | linux* | vt*)
        _ansi_escapes_are_valid=true
        ;;
      esac
    fi
  fi

  return_value="$_ansi_escapes_are_valid"
}

info() {
  ansi_escapes_are_valid
  local _ansi_escapes_are_valid="$return_value"

  local _green='\033[0;32m'
  local _no_color='\033[0m'

  if $_ansi_escapes_are_valid; then
    # shellcheck disable=SC2059
    printf "  ${_green} INFO${_no_color} $1\n"
  else
    echo "   INFO $1"
  fi
}

error() {
  ansi_escapes_are_valid
  local _ansi_escapes_are_valid="$return_value"

  local _red='\033[0;31m'
  local _no_color='\033[0m'

  echo

  if $_ansi_escapes_are_valid; then
    # shellcheck disable=SC2059
    printf "  ${_red}ERROR${_no_color} $1\n"
  else
    echo "  ERROR $1"
  fi

  echo
  echo "   If you need help, please start a discussion on Github:"
  echo "   https://github.com/build-trust/ockam/discussions/new"
  echo

  exit 1
}

heading() {
  ansi_escapes_are_valid
  local _ansi_escapes_are_valid="$return_value"

  local _orange='\033[0;33m'
  local _no_color='\033[0m'

  if $_ansi_escapes_are_valid; then
    # shellcheck disable=SC2059
    printf "${_orange}   $1${_no_color}\n"
  else
    echo "   $1"
  fi
}

required() {
  if ! command -v "$1" >/dev/null 2>&1; then
    error "need '$1' (command not found)"
  fi
}

display_usage() {
  echo "ockam/install.sh - installs the ockam binary"
  echo " "
  echo "Usage: install.sh [OPTION]..."
  echo "    -h, --help                show brief help"
  echo "    -p, --install-path PATH   specify the location for installation"
  echo "                              (default path is ~/.ockam)"
  echo "    -v, --version VERSION     specify the version to install"
  echo "        --no-modify-path      do not add ockam to the PATH"
}

# using custom function instead of eval due to associated risks of using
# eval to expand user inputted filepath
expand_filepath() {
  local _path="$1"
  _path="${_path/#\./$(pwd)}"
  _path="${_path/#\~/$HOME}"
  return_value="$_path"
}

# replace HOME if it exists in the install path to reduce brittleness
sub_path_home() {
  required sed

  local _path="$1"
  # _path=$(echo "$_path" | sed "s|$HOME|\$HOME|")
  _path="${_path//$HOME/\$HOME}"
  return_value="$_path"
}

detect_binary_file_name() {
  info "Detecting Operating System and Architecture ..."

  local _os_type _cpu_type _file_name
  _os_type="$(uname -s)"
  _cpu_type="$(uname -m)"

  case "$_os_type" in
  Darwin)
    if [ "$_cpu_type" = i386 ]; then
      # Darwin `uname -m` lies
      if sysctl hw.optional.x86_64 | grep -q ': 1'; then
        _cpu_type=x86_64
      fi
    fi

    case "$_cpu_type" in
    x86_64) _file_name="ockam.x86_64-apple-darwin" ;;
    arm64) _file_name="ockam.aarch64-apple-darwin" ;;
    *) error "Unsupported CPU type: ${_cpu_type} on MacOS" ;;
    esac
    ;;
  Linux)
    case "$_cpu_type" in
    x86_64 | aarch64) _file_name="ockam.$_cpu_type-unknown-linux-musl" ;;
    armv7l) _file_name="ockam.$_cpu_type-unknown-linux-musleabihf" ;;
    *) error "Unsupported CPU type: ${_cpu_type} on Linux" ;;
    esac
    ;;
  *) error "Unsupported operating system type: ${_os_type}" ;;
  esac

  info "Detected Operating System Type: ${_os_type}"
  info "Detected CPU Type: ${_cpu_type}"
  info "Picked Released File Name: ${_file_name}"

  return_value="$_file_name"
}

download() {
  required curl
  required grep
  required awk
  required sed

  local _version _url
  local _download_base_url="https://github.com/build-trust/ockam/releases/download"
  local _api='https://api.github.com/repos/build-trust/ockam/releases'
  local _binary_file_name="$1"

  if [ "$2" ]; then
    _version="$2"
    _url="$_download_base_url/ockam_$_version/$_binary_file_name"

    info "Installing $_version"
  else
    _url="https://github.com/build-trust/ockam/releases/latest/download/$_binary_file_name"

    info "Installing latest version"
  fi

  info "Downloading $_url"
  curl --proto '=https' --tlsv1.2 --location --silent --fail --show-error --output "$install_path/bin/ockam" "$_url"
  info "Downloaded ockam binary at the specified directory: $install_path/bin/ockam"

  info "Granting permission to execute: chmod u+x $install_path/bin/ockam"
  chmod u+x "$install_path/bin/ockam"
}

create_bin() {
  info "Creating binary directory at specified install path"
  if [[ -f "$install_path" ]]; then
    error "$install_path already exists but is not a directory"
    exit 1
  fi

  if [[ -f "$install_path/bin" ]]; then
    error "$install_path/bin already exists but is not a directory"
    exit 1
  fi

  mkdir -p "$install_path/bin"
  info "Binary directory successfully created"
}

write_env_files() {
  info "Setting up env script"

  local _ockam_env="$install_path/env"
  if [[ -d "$_ockam_env" ]]; then
    error "$_ockam_env already exists but is not a file"
    exit 1
  fi

  local _ockam_bin="$install_path/bin"

  if [[ ! -d $_ockam_bin ]]; then
    error "Failed to find binary directory at: $_ockam_bin"
    exit 1
  fi

  sub_path_home "$_ockam_bin"
  _ockam_bin="$return_value"

  echo "
#!/bin/sh
# ockam shell setup
# affix colons on either side of \$PATH to simplify matching
case \":\${PATH}:\" in
    *:\"$_ockam_bin\":*)
        ;;
    *)
        # Prepending path in case a system-installed ockam needs to be overridden
        export PATH=\"$_ockam_bin:\$PATH\"
        ;;
esac" >"$_ockam_env"

  info "env script was successfully written"
}

add_to_path() {
  info "Adding Ockam to PATH"
  local _ockam_env="$install_path/env"

  sub_path_home "$_ockam_env"
  local _ockam_env_sub_home="$return_value"
  local _source_cmd=". \"$_ockam_env_sub_home\""

  local _rcfiles=('.profile' '.bash_profile' '.bash_login' '.bashrc' '.zshenv')

  info "Sourcing env script into rcfiles"
  for rcfile in "${_rcfiles[@]}"; do
    if [[ ! -f "$_ockam_env" ]]; then
      error "env script missing, expected script at $_ockam_env"
      exit 1
    fi

    local _rcpath="$HOME/$rcfile"

    if [[ ! -f "$_rcpath" ]]; then
      continue
    fi

    local _contents
    _contents=$(cat "$_rcpath")
    if [[ "$_contents" == *"$_source_cmd"* ]]; then
      continue
    fi

    info "Adding source command to $_rcpath"
    echo >>"$_rcpath"
    echo "$_source_cmd" >>"$_rcpath"
  done
}

main() {
  local _version=""
  install_path="$HOME/.ockam"
  local _modify_path="true"

  while test "$#" -gt 0; do
    case "$1" in
    -h | --help)
      shift
      display_usage
      exit 0
      ;;

    -p | --install-path)
      shift
      if test $# -gt 0; then
        expand_filepath "$1"
        install_path="$return_value"
      else
        display_usage
        exit 1
      fi
      shift
      ;;

    -v | --version)
      shift
      if test $# -gt 0; then
        _version="$1"
      else
        display_usage
        exit 1
      fi
      shift
      ;;

    --no-modify-path)
      shift
      _modify_path="false"
      ;;

    *)
      echo "Invalid parameter: $1"
      display_usage
      exit 1
      ;;
    esac
  done

  echo
  info "Installing Ockam Command ..."

  detect_binary_file_name
  local _binary_file_name="$return_value"

  create_bin
  download "$_binary_file_name" "$_version"

  write_env_files

  if [[ "$_modify_path" == "true" ]]; then
    add_to_path
  fi

  echo
  heading "GET STARTED:"

  sub_path_home "$install_path"
  install_path="$return_value"

  if [[ "$_modify_path" == "false" ]]; then
    echo "   Ockam Command is now installed at: \"$install_path/bin/ockam\""
    echo
    echo "   You can execute it by running:"
    echo "     \"$install_path/bin/ockam\""
    echo
    echo "   If you wish to run it from anywhere on your machine ..."
    echo
    echo "   Please add \"$install_path/bin\" to your system \$PATH"
    echo
    echo "   After that, you should be able to execute it anywhere by simply typing:"
    echo "     ockam"
  else
    echo "   Ockam Command is installed at: \"$install_path/bin/ockam\""
    echo
    echo "   To get started you may need to restart your current shell. This would"
    echo "   reload your PATH environment variable to include Ockam's bin directory: \"$install_path/bin\""
    echo
    echo "   To configure your current shell, run:"
    echo "     source \"$install_path/env\""
  fi
  echo
  heading "LEARN MORE:"
  echo "   Learn more at https://docs.ockam.io"
  echo
  heading "FEEDBACK:"
  echo "   If you have any questions or feedback, please start a discussion"
  echo "   on Github https://github.com/build-trust/ockam/discussions/new"
  echo

  exit 0
}

main "$@"

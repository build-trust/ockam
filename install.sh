#!/bin/sh
# shellcheck shell=dash

set -e

# This script fetches precompiled released Ockam binaries and
# stores them in the current directory.
# https://github.com/build-trust/ockam/releases

# You can call it as follow:
#
# To install the latest released version:
# curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/build-trust/ockam/develop/install.sh | sh
#
# Or
#
# To install a specific version:
# curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/build-trust/ockam/develop/install.sh | sh -s -- v0.74.0

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
    echo "  ${_green} INFO${_no_color} $1"
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
    echo "   ${_red}ERROR${_no_color} $1"
  else
    echo "   ERROR $1"
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
    echo "${_orange}        $1${_no_color}"
  else
    echo "${_orange}        $1${_no_color}"
  fi
}

required() {
  if ! command -v "$1" >/dev/null 2>&1; then
    error "need '$1' (command not found)"
  fi
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
  curl --proto '=https' --tlsv1.2 --location --silent --fail --show-error --output "ockam" "$_url"
  info "Downloaded ockam command in the current directory $(pwd)"

  info "Granting permission to execute: chmod u+x ockam"
  chmod u+x ockam
}

main() {
  echo
  info "Installing Ockam Command ..."

  local _version="$1"

  detect_binary_file_name
  local _binary_file_name="$return_value"

  download "$_binary_file_name" "$_version"

  echo
  heading "GET STARTED:"
  echo "        Ockam Command is ready to be executed in the current directory."
  echo
  echo "        You can execute it by running:"
  echo "          ./ockam"
  echo
  echo "        If you wish to run it from anywhere on your machine ..."
  echo
  echo "        Please copy it to a directory that is in your \$PATH, for example:"
  echo "          mv ockam /usr/local/bin"
  echo
  echo "        After that, you should be able to execute it anywhere by simply typing:"
  echo "          ockam"
  echo
  heading "LEARN MORE:"
  echo "        Learn more at https://docs.ockam.io/get-started#command"
  echo
  heading "FEEDBACK:"
  echo "        If you have any questions or feedback, please start a discussion"
  echo "        on Github https://github.com/build-trust/ockam/discussions/new"
  echo

  exit 0
}

main "$1"

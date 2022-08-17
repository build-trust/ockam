# Ensure all executables are installed
executables_installed=true
if ! command -v jq &> /dev/null; then
  echo "JQ executable not installed. Please install at https://stedolan.github.io/jq/"
  executables_installed=false
fi
if ! command -v gh &> /dev/null; then
  echo "Github CLI not installed. Please install at https://cli.github.com"
  executables_installed=false
fi

if [[ $executables_installed == false ]]; then
  echo "Required executables not installed. Exiting now."
  exit 1
fi
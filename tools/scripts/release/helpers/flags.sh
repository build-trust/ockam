while getopts drhf flag
do
    case "${flag}" in
        h) SHOW_HELP=true;;
        r) IS_DRAFT_RELEASE=false;;
        d) IS_DRAFT_RELEASE=true;;
        f) RECENT_FAILURE=true;;
    esac
done

if [[ ! -z $SHOW_HELP ]]; then
  echo "This script automates Ockam release for which consists of
  - Binary release
  - Homebrew release
  - Crate release
  - Terraform release
  - Docker package release
To run the script, there are flags that can be passed
  -f - Indicates a recent failure was was made during a recent release so that the script can pickup from last fail.
  -d - Indicates script to create \`DRAFT\` release
  -r - Indicates script to create production release from recently generated draft
  -h - Shows help message"

  exit 0
fi

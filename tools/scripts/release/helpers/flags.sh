RECENT_FAILURE=false

while getopts drhf: flag
do
    case "${flag}" in
        h) SHOW_HELP=true;;
        r) IS_DRAFT_RELEASE=false;;
        d) IS_DRAFT_RELEASE=true;;
        f) RECENT_FAILURE=true && LATEST_TAG_NAME=${OPTARG};;
    esac
done

if [[ $IS_DRAFT_RELEASE == true && $RECENT_FAILURE == true ]]; then
  echo "Cannot run script in failure mode for draft release"
  exit 1
fi

if [[ ! -z $LATEST_TAG_NAME && $LATEST_TAG_NAME != *"ockam_v"* ]]; then
  echo "Indicated tag is invalid"
  exit 1
fi

if [[ ! -z $SHOW_HELP ]]; then
  echo "This script automates Ockam release which consists of
  - Binary release
  - Homebrew release
  - Crate release
  - Terraform release
  - Docker package release
To run the script, there are flags that can be passed
  -f TAG_NAME - Indicates a recent failure was made during a recent release so that the script can pickup from last fail. You need to indicate the failed release tag.
  -d - Indicates script to create \`DRAFT\` release
  -r - Indicates script to create \`PRODUCTION\` release from recently generated draft
  -h - Shows this help message"

  exit 0
fi

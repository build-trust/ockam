#!/bin/bash

# Example Usage:
# cargo deny --all-features list --config=tools/cargo-deny/deny.toml --layout=crate --format json | ./tools/scripts/release/parseCrates.sh

OLD_CKSUM=$(cat NOTICE.md | cksum) # cksum old file, compare value to new below

intro_text="This file contains attributions for any 3rd-party open source code used in this project."
table_header="| Name | License | URL |\n|------|---------|-----|\n"

# Append table header to the table content
table_content="$table_header"

# Derive input
INPUT=$(jq "." $1)

# Sample string that will be matched:
# "adler 1.0.2 registry+https://github.com/rust-lang/crates.io-index"
regex="(.*) [0-9]+\.[0-9]+\.[0-9]+ \(*(.*)\+(.*)"

while IFS= read -r key; do
  if [[ $key =~ $regex ]]; then

    crate_name=${BASH_REMATCH[1]}
    crate_source=${BASH_REMATCH[2]}
    url=${BASH_REMATCH[3]}
    license=$(jq --arg key "$key" --raw-output '.[$key].licenses | join(", ")' <<<$INPUT)

    # Strip URL of trailing )
    url="${url//\)/}"

    if [[ "$url" == *"crates.io"* ]]; then
      url="https://crates.io/crates/${crate_name}"
    fi

    # Ignore crates pulled from path; they are either examples or Ockam crates
    if [[ "$crate_source" == "path" ]]; then
      continue
    else
      # Append crate data to table
      table_content+="| $crate_name | $license | $url |\n"
    fi
  fi
done < <(jq --raw-output "keys[]" <<<"$INPUT") # avoids creation of a subshell with pipe (`|`)

# Combine introductory text and table content
file_content="$intro_text\n\n$table_content"

# Comment out next 6 lines to disable cksum checking
NEW_CKSUM=$(echo -e "$file_content"$'\n' | cksum) # cksum new results, compare to old

if [[ $OLD_CKSUM != $NEW_CKSUM ]]; then
  #TO-DO Add automated PR to update NOTICE file

  echo "NOTICE file update required."
  echo "NOTICE file updating..."

  # Save new notice file
  echo -e "$file_content"$'\n' >NOTICE.md
  echo "NOTICE file updated."
else
  echo "Notice file is up to date."
fi

# NOTE: Uncomment to save output w/o checking cksum values
# echo -e "$file_content"$'\n' > NOTICE

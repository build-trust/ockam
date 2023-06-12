#!/bin/bash

# Example Usage: 
# cargo deny --all-features list --config=tools/cargo-deny/deny.toml --layout=crate --format json | ./parseCrates.sh > NOTICE

OPEN_BRACKET=$(expr 0)      # count open brackets, diff between a license name and library name
                            # set to 1 as we remove the 1st instance before counting
LICENSE_STRING=""
FILEOUT=""
PROJECT=""
VERSION=""
URL=""

OLD_CKSUM=$(cat NOTICE | cksum)         # cksum old file, compare value to new below

FILEOUT=$(printf "%25s %40s" "Crate Name" "License" )
FILEOUT+=$(printf "\n------------------------------------------------------------------")

# Derive input
INPUT=$(jq "." $1 | tr -d '{},()":')

# Set IFS to newline
IFS=$'\n'

for line in $INPUT
do 

    # We have the closing bracket, set OPEN_BRACKET to 0, remove any leading commas from
    # licensing, and build output for this library & licensing
    # NOTE: Important this check is before check for '[', otherwise crates w/o license 
    # are presented incorrectly
    if [[ $line == *"]"* ]]; then
        IFS=$' '
        OPEN_BRACKET=$(expr 0)      # reset OPEN_BRACKET status
        LICENSE_STRING="${LICENSE_STRING#,}"    # remove any leading comma
        LICENSE_STRING=$(echo $LICENSE_STRING | sed 's/ //g')       # remove leading spaces from LICENSES
        LICENSE_STRING=$(echo $LICENSE_STRING | sed 's/,/, /g')     # add space after commas in LICENSES
        FILEOUT+=$(printf "\n%25s %40s" "$PROJECT" "$LICENSE_STRING")
        LICENSE_STRING=""           # reset LICENSE_STRING
        IFS=$'\n'
        continue        # skip to next iteration/line
    fi

    # closing bracket, lets subtract from OPEN_BRACKET
    if [[ $line == *"licenses ["* ]]; then
        OPEN_BRACKET=$(expr 1)
        continue        # skip to next iteration/line
    fi

    # Keep appending license names until we hit closing bracket
    if [[ $OPEN_BRACKET -eq 1 ]]; then
        LICENSE_STRING+=", $line"
        continue        # skip to next iteration/line
    fi
    
    # This line contains our library, version, and URL
    if [[ $OPEN_BRACKET -eq 0 ]] && [[ ${#line} -gt 3 ]]; then
        IFS=$' '
        read -r PROJECT VERSION URL <<<"$line"
        PROJECT=$(echo "$PROJECT" | sed 's/"//g')
        VERSION=$(echo "$VERSION" | sed 's/"//g')
        URL=$(echo "$URL" | sed 's/"//g')
        URL="${URL/registry+}"      # remove 'registry+' from start of URL
        IFS=$'\n'
    fi

done

# Comment out next 6 lines to disable cksum checking
NEW_CKSUM=$(printf "%s\n" $FILEOUT | cksum)     # cksum new results, compare to old

if [[ $OLD_CKSUM != $NEW_CKSUM ]]; then
    # Save new notice file
    printf "%s\n" "$FILEOUT"        # Note: could not get echo to parse line breaks
fi

# NOTE: Uncomment to save output w/o checking cksum values
# printf "%s\n" "$FILEOUT"        # Note: could not get echo to parse line breaks



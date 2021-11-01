#!/usr/bin/env bash

# Crates to be bumped. In inter-dependency order.
declare -A modified_crates;

# Crates should be appended in non-interdependent format.
# signature_core is not dependent on anyother crate.
crates=(
    "signature_core"
    "signature_bls"
    "ockam_vault_test_attribute"
    "ockam_node_no_std"
    "ockam_node_attribute"
    "ockam_core"
    "signature_ps"
    "signature_bbs_plus"
    "ockam_vault_core"
    "ockam_vault_test_suite"
    "ockam_executor"
    "ockam_node"
    "ockam_transport_core"
    "ockam_transport_tcp"
    "ockam_transport_websocket"
    "ockam_key_exchange_core"
    "ockam_vault"
    "ockam_vault_sync_core"
    "ockam_key_exchange_x3dh"
    "ockam_key_exchange_xx"
    "ockam_ffi"
    "ockam_channel"
    "ockam_entity"
    "ockam"
);

# TODO: We use git push -f not sure if this is safe.


if [[ -z $TOKEN ]]; then
    echo "$(tput setaf 1)Token not specified$(tput sgr0)"
    exit 1;
fi

if [[ -z $UPSTREAM ]]; then
    echo "$(tput setaf 1)Upstream branch to compare diff changes not specified$(tput sgr0)"
    exit 1;
fi

if git rev-parse --verify "$UPSTREAM"; then
    echo "$(tput setaf 2)upstream branch $UPSTREAM exists $(tput sgr0)";
else
    echo "$(tput setaf 1)upstream branch $UPSTREAM does not exists $(tput sgr0)";
    exit 1;
fi


is_folder_updated(){
    cd "../../../implementations/rust/ockam/$1";

    updated=1;
    git diff "$UPSTREAM" --quiet --name-status -- ./src || updated=0;

    # Check if Cargo.toml was changed.
    if [[ $updated == 1 ]]; then
        git diff "$UPSTREAM" --quiet --name-status -- ./Cargo.toml || updated=0;
    fi

    echo $updated
}

# We try to strip the version from changelog
# ## v0.36.0 - 2021-10-29 should strip out just the
# version. But if we have same date for different versions
# could cause issues.
# TODO: this could go wrong if we push >1 same day.
get_changelog_version(){
    query="## v[^\"]* - $1";
    version=$(eval "grep -w -m 1 '$query' CHANGELOG.md  | sed -e 's/^##\ //' -e 's/ - $1//' ");
    echo $version
}




if cargo install cargo-release; then
    echo "$(tput setaf 2)cargo release installed$(tput sgr0)\n\n"
else
    echo "$(tput setaf 1)Failed to install cargo releaser$(tput sgr0)";
    exit 1;
fi;



current_commit=$(eval "git rev-parse HEAD");
echo "current commit $current_commit";

# Bump only crates that have been change with respech to
# upstream branch.
# Since cargo release binary also updates crates that are
# interdependent, we can also updates crates whose Cargo.toml
# has also been updated.
for d in "${crates[@]}"; do
    crate_updated="$(is_folder_updated $d)";
    (
        if [[ $crate_updated == 0 ]]; then
            cd "../../../implementations/rust/ockam/$d";
            if echo y | cargo  release minor --skip-tag --skip-push --skip-publish --no-dev-version --execute -q; then
                echo "$(tput setaf 2)Bumped $d crate$(tput sgr0)\n\n";
            else
                git reset $current_commit;
                read -p "crate $d bump failed. Press enter to continue or ctrl-c to exit:   ";

                git add --all;
                git commit -m "committed all crates";
            fi
        fi
    )
    if [[ $crate_updated == 0 ]]; then
        # Store modified crates name to store.
        modified_crates[$d]=true;
    fi
done

if [[ ${#modified_crates[@]} == 0 ]]; then
    echo "$(tput setaf 2)${#modified_crates[@]} no crate was updated. exiting$(tput sgr0)\n\n";
    exit 0;
fi



# Push commits upstream.
if git reset $current_commit; then
    echo "$(tput setaf 2)cargo release commits squashed$(tput sgr0)\n\n";
else
    echo "$(tput setaf 2)error reseting cargo release commits$(tput sgr0)\n\n";
    exit 1;
fi

read -p "Please confirm git diff and press enter to push to upstream branch or ctrl-c to abort:   ";

commit_message="feat(rust): crate release $(date +'%b %d %Y')";

if git add --all; then
    echo "$(tput setaf 2)'git added' all files$(tput sgr0)";
else
    echo "$(tput setaf 2)error calling git add$(tput sgr0)";
    exit 1;
fi

if git commit -m "$commit_message"; then
    echo "$(tput setaf 2)successfully committed crate bump$(tput sgr0)";
else
    echo "$(tput setaf 2)error committing crate bump$(tput sgr0)";
    exit 1;
fi

# Push commits to current branch so that it can be merged to main.
current_branch=$(eval "git rev-parse --abbrev-ref HEAD");
if git push --set-upstream origin "$current_branch" -f; then
    echo "$(tput setaf 2)successfully pushed to upstream$(tput sgr0)";
else
    echo "$(tput setaf 2)error pushing commits upstream$(tput sgr0)";
    exit 1;
fi

read -p "Commits pushed upstream, please merge and press enter to start git tagging or ctrl-c to abort:  ";




# Git tag only crates that were bumped.
current_date=$(eval "date +'%Y-%m-%d'");

for d in "${crates[@]}"; do
    (
        if [ -z "${modified_crates[$d]}" ]; then
            echo "$(tput setaf 2)$d crate will not be tagged as it not modified$(tput sgr0)";

        else
            cd "../../../implementations/rust/ockam/$d";
            # TODO: We can store all versions DRY.
            version=$(get_changelog_version $current_date);
            echo "version is $version";
            stripped_version="${version//v}";

            if [ -z "$version" ]; then
                read -p "error finding version for crate $d to tag press ctrl-c to exit or enter to continue:  ";
            else
                tag="${d}_${version}"
                echo "$(tput setaf 2)Tagging $tag$(tput sgr0)";

                text="* [Crate](https://crates.io/crates/$d/$stripped_version)
* [Documentation](https://docs.rs/$d/$stripped_version/$d/)
* [CHANGELOG](https://github.com/ockam-network/ockam/blob/${d}_$version/implementations/rust/ockam/$d/CHANGELOG.md)";

                if gh release create --notes "$text" -t "$d $version (rust crate)" "$tag"; then
                    echo "$(tput setaf 2)$d $version crate git tagged$(tput sgr0)";
                else
                    read -p "error tagging $d crate. Press ctrl-c to exit or enter to continue:  ";
                fi
            fi
        fi
    )
done




# Cargo publish only crates that were bumped.
for d in "${crates[@]}"; do
    (
        if [ -z "${modified_crates[$d]}" ]; then
            echo "$(tput setaf 2)$d crate will not be published as it is not modified$(tput sgr0)";

        else
            cd "../../../implementations/rust/ockam/$d";
            version=$(get_changelog_version $current_date);
            stripped_version="${version//v}";

            if [ -z "$version" ]; then
                read -p "error finding version for crate $d to tag press ctrl-c to exit or enter to continue:  ";
            else
                # Publish crate online and wait till crate is seen crates.io.
                if cargo publish --token "$TOKEN"; then
                    # We keep on checking crates.io till crate is uploaded.
                    while :
                    do
                        echo "$(tput setaf 2)Checking if $d crate has been published$(tput sgr0)";
                        if cargo search -q $d | grep -w "$d = \"$stripped_version\""; then
                            echo "$(tput setaf 2)Published crate $d seen on crates.io$(tput sgr0)";
                            sleep 2;
                            break;
                        else
                            echo "$(tput setaf 2)Published crate has not been updated, retrying in 2 sec$(tput sgr0)";
                            sleep 2;
                        fi
                    done

                else
                    read -p "error publishing crate $d press ctrl-c to exit or enter to continue:  ";
                fi
            fi
        fi
    )
done

echo "Crates Publishing successful. Crates published are \n ${modified_crates[@]}";

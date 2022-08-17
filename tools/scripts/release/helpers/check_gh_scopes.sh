# Check GH API Token scope to ensure they have required scope

api_response=$(eval gh api -i /)

regex="X-Oauth-Scopes: ([^X-]*)"
if [[ $api_response =~ $regex ]]; then
    if [[ ${BASH_REMATCH[1]} != *"delete:packages"* ]]; then
        echo "Release script requires delete:package scope"
        exit 1
    fi
else
    echo "Error finding Github Auth scope"
    exit 1
fi

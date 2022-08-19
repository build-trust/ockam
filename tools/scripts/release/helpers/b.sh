DONE="Yes"
source tools/scripts/release/helpers/a.sh

subfolders="./general/users ./general/eks_service_accounts"

for folder in ${subfolders[@]}; do
    echo "$folder"
done
HELLO="HHH"
echo "$WHAT"
echo "----- $SHOULDNT_SEE"
dd "Hello"

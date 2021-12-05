git merge-base --is-ancestor 2fd0d36fe6ae0c2d527368683ec3a6352617b381 HEAD || \
  (echo '
    This workflow checks that all commit follow the Ockam Commit Message Convention
    https://ockam.io/learn/how-to-guides/contributing/CONTRIBUTING#commit-messages


  ' && exit 1)

npx commitlint \
  --config tools/commitlint/commitlint.config.js \
  --from 2fd0d36fe6ae0c2d527368683ec3a6352617b381 \
  --to HEAD \
  --help-url https://www.ockam.io/learn/how-to-guides/contributing/CONTRIBUTING#commit-messages || \
  (echo '
    This workflow checks that all commits follow the Ockam Commit Message Convention
    https://ockam.io/learn/how-to-guides/contributing/CONTRIBUTING#commit-messages
  ' && exit 1)

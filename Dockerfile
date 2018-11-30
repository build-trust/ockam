
# An image with node v10.13.0 and git for use by other node based tool images as a base stage.
# Also sets the working directory to /project.
FROM node:10.13.0-alpine@sha256:22c8219b21f86dfd7398ce1f62c48a022fecdcf0ad7bf3b0681131bd04a023a2 as node
RUN apk update && apk add git
WORKDIR /project
ENTRYPOINT ["node"]

# An image that runs commitlint in the /project working directory.
# https://github.com/marionebl/commitlint
#
# DOCKER_BUILDKIT=1 docker build --target commitlint --tag ockam/tool/commitlint:latest .
# docker run --rm --volume "$(pwd):/project" ockam/tool/commitlint:latest --from=HEAD~1
FROM node as commitlint
RUN npm install --global @commitlint/config-conventional@7.1.2 @commitlint/cli@7.2.1
ENTRYPOINT ["commitlint"]
CMD ["--from=HEAD~1"]

# An image that validates the contents of the /project working directory against
# rules in .editorconfig files present in that directory tree.
# https://editorconfig.org
# https://github.com/jedmao/eclint
#
# DOCKER_BUILDKIT=1 docker build --target eclint --tag ockam/tool/eclint:latest .
# docker run --rm --volume "$(pwd):/project" ockam/tool/eclint:latest
FROM node as eclint
RUN npm install --global eclint@2.8.1
ENTRYPOINT ["eclint"]
CMD ["check"]

# An image that invokes shellcheck on any file path that is passed as an argument.
# This path must be relative to the mounted /project directory.
# https://www.shellcheck.net
#
# DOCKER_BUILDKIT=1 docker build --target shellcheck --tag ockam/tool/shellcheck:latest .
# docker run --rm --volume "$(pwd):/project" ockam/tool/shellcheck:latest build
FROM koalaman/shellcheck:v0.5.0@sha256:b8a2b097586f88578d45ac9c052d7c396fe651a128e44ab99b3742851033b9f5 as shellcheck
WORKDIR /project
ENTRYPOINT ["/bin/shellcheck"]
CMD ["-a", "build"]

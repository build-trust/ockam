
# An image with node v10.14.1 and git for use by other node based tool images as a base stage.
# Also sets the working directory to /project.
FROM node:10.14.1-alpine@sha256:35fcf0a48f57bef4bafa0f844f62edb659d036364a1d086995efe5b43ca0c4af as node
RUN apk --update add --no-cache git
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

# An image with Golang v1.11.2 and git.
# It sets /project as the working directory and runs Go as its entrypoint
#
# DOCKER_BUILDKIT=1 docker build --target go --tag ockam/tool/go:latest .
# docker run --rm --volume "$(pwd):/project" ockam/tool/go:latest
FROM golang:1.11.2-alpine3.8@sha256:692eff58ac23cafc7cb099793feb00406146d187cd3ba0226809317952a9cf37 as go
ENV GOOS=linux GOARCH=amd64
RUN apk --update add --no-cache git
WORKDIR /project
ENTRYPOINT ["go"]

# An image that invokes gometalinter in the /project directory
#
# DOCKER_BUILDKIT=1 docker build --target gometalinter --tag ockam/tool/gometalinter:latest .
# docker run --rm --volume "$(pwd):/project" ockam/tool/gometalinter:latest
#
# gometalinter doesn't support go modules yet, the /entrypoint script below tries to make project
# as if it was following the pre-1.11 GOPATH setup.
FROM go as gometalinter
RUN wget https://raw.githubusercontent.com/alecthomas/gometalinter/v2.0.11/scripts/install.sh \
		&& chmod u+x install.sh && ./install.sh -b /usr/local/bin v2.0.11 \
		&& mkdir -p /go/src/github.com/ockam-network/ockam \
		&& echo "#!/bin/sh" > /entrypoint \
		&& echo "cp -r /project/* /go/src/github.com/ockam-network/ockam/" >> /entrypoint \
		&& echo "rm -rf /go/src/github.com/ockam-network/ockam/vendor" >> /entrypoint \
		&& echo "cp -r /project/vendor/* /go/src/" >> /entrypoint \
		&& echo "exec gometalinter \"\$@\"" >> /entrypoint \
		&& chmod +x /entrypoint
WORKDIR /go/src/github.com/ockam-network/ockam
ENV GO111MODULE=off
ENTRYPOINT ["/entrypoint"]
CMD ["--vendor", "--enable-all", "--line-length=120", "./..."]

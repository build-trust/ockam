FROM debian:10-slim
RUN apt-get -y update
RUN apt-get -y install libterm-readline-gnu-perl
RUN apt-get -y install apt-utils
RUN apt-get -y upgrade
RUN apt-get -y install clang lld curl git
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
COPY tmpl/setup.sh /
COPY tmpl/get_started.sh /
COPY tmpl/website_guide.sh /
COPY tmpl/example_projects.sh /
COPY tmpl/entry.sh /
RUN /setup.sh
ENTRYPOINT "/entry.sh"


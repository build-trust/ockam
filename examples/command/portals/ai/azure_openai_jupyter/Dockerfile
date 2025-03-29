FROM quay.io/jupyter/datascience-notebook

USER root
RUN apt update && apt upgrade --yes
RUN conda update conda && conda install python=3.12 --yes --no-pin
RUN apt-get install software-properties-common jq dnsutils --yes && apt update

RUN pip install --no-cache-dir 'openai' 'jupyter-ai[all]' && \
    fix-permissions "${CONDA_DIR}" && \
    fix-permissions "/home/${NB_USER}"

COPY ./notebook_setup/run_ockam.sh /usr/local/bin/start-notebook.d/001_run_ockam.sh
RUN chmod +x /usr/local/bin/start-notebook.d/001_run_ockam.sh

USER $NB_UID

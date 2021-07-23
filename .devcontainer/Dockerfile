# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.187.0/containers/rust/.devcontainer/base.Dockerfile

FROM mcr.microsoft.com/vscode/devcontainers/rust:0-1

# [Optional] Uncomment this section to install additional packages.
# RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
#     && apt-get -y install --no-install-recommends <your-package-list-here>

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt upgrade \
    && rustup update && \
    && cargo install sccache && \
    && echo -e "[build]\nrustc-wrapper = \"/usr/local/cargo/bin/sccache\"\n" > /usr/local/cargo/config.toml && \
    && cargo install cargo-audit cargo-outdated cargo-install-update cargo-watch cargo-crev && \
    && sccache --stop-server && \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/* /usr/local/cargo/registry ~/.cache/sccache

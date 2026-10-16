# The WinWeaver assistant in the pi agent. Every build takes the latest Ubuntu LTS, the latest Node.js LTS and the
# latest pi. The image holds nothing of WinWeaver but the instructions and skills of the assistant; pi reaches the
# program only through its MCP server. Setup: README.md next to this file.
#
# Build from the folder agents (the parent of this one), so that the instructions and skills are in the context:
#   podman build --pull --no-cache -t winweaver-pi -f pi-agent/Dockerfile .
# Run:
#   podman run -it --rm --name winweaver-pi -v pi-winweaver:/home/pi/.pi --network=host winweaver-pi

FROM docker.io/library/ubuntu:latest

ENV DEBIAN_FRONTEND=noninteractive

# bash for pi's bash tool and the person's own "!" commands; ripgrep and fd-find, which pi looks for at every start;
# curl for the downloads below.
RUN apt-get update && \
    apt-get install -y --no-install-recommends bash ca-certificates curl ripgrep fd-find && \
    rm -rf /var/lib/apt/lists/*

# The latest Node.js LTS from nodejs.org (the first release of index.tab with an LTS name), checked against the
# SHASUMS256.txt of that release. Not a distribution package: it may lag behind what pi needs.
RUN arch="$(dpkg --print-architecture)" && \
    case "$arch" in amd64) node_arch=x64 ;; arm64) node_arch=arm64 ;; \
        *) echo "no Node.js build for $arch" >&2; exit 1 ;; esac && \
    version="$(curl -fsSL https://nodejs.org/dist/index.tab | awk -F'\t' 'NR > 1 && $10 != "-" { print $1; exit }')" && \
    archive="node-${version}-linux-${node_arch}.tar.gz" && \
    cd /tmp && \
    curl -fsSLO "https://nodejs.org/dist/${version}/${archive}" && \
    curl -fsSL "https://nodejs.org/dist/${version}/SHASUMS256.txt" | grep "  ${archive}\$" | sha256sum -c - && \
    mkdir -p /opt/node && \
    tar -xzf "${archive}" -C /opt/node --strip-components=1 --no-same-owner && \
    rm -f "${archive}"
ENV PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/opt/node/bin

# The latest pi; --ignore-scripts as its documentation installs it, so no package script runs during the build.
RUN npm install -g --ignore-scripts @earendil-works/pi-coding-agent@latest && \
    npm cache clean --force && \
    node --version && pi --version

ENV HOME=/home/pi
ENV PI_CODING_AGENT_DIR=/home/pi/.pi/agent
# No install ping and no version check: the image is rebuilt to update.
ENV PI_TELEMETRY=0
ENV PI_SKIP_VERSION_CHECK=1

# pi loads every AGENTS.md from the working folder up to the root: the instructions for any agent at /, the pi part in
# /work. The skills go to the user skills folder outside the volume, so the image keeps them current.
COPY AGENTS.md /AGENTS.md
COPY pi-agent/AGENTS.md /work/AGENTS.md
COPY skills/ /home/pi/.agents/skills/
RUN mkdir -p /home/pi/.pi/agent && chmod -R a+rwX /home/pi /work

WORKDIR /work
CMD ["pi"]

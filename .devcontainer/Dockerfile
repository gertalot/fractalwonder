FROM node:20

ARG TZ
ENV TZ="${TZ:-America/Los_Angeles}"

ARG CLAUDE_CODE_VERSION=latest

# Install basic development tools and Chrome for browser testing
RUN apt-get update && apt-get install -y --no-install-recommends \
  less \
  git \
  procps \
  sudo \
  fzf \
  zsh \
  man-db \
  unzip \
  gnupg2 \
  gh \
  jq \
  nano \
  vim \
  build-essential \
  pkg-config \
  libssl-dev \
  curl \
  socat \
  locales \
  chromium \
  chromium-driver \
  && apt-get clean && rm -rf /var/lib/apt/lists/*

# Generate and set locale
RUN sed -i -e 's/# en_US.UTF-8 UTF-8/en_US.UTF-8 UTF-8/' /etc/locale.gen && \
  locale-gen
ENV LANG=en_US.UTF-8 \
  LANGUAGE=en_US:en \
  LC_ALL=en_US.UTF-8

# Ensure default node user has access to /usr/local/share
RUN mkdir -p /usr/local/share/npm-global && \
  chown -R node:node /usr/local/share

ARG USERNAME=node

# Persist bash history
RUN SNIPPET="export PROMPT_COMMAND='history -a' && export HISTFILE=/commandhistory/.bash_history" \
  && mkdir /commandhistory \
  && touch /commandhistory/.bash_history \
  && chown -R $USERNAME /commandhistory

# Set `DEVCONTAINER` environment variable to help with orientation
ENV DEVCONTAINER=true

# Create workspace and config directories and set permissions
RUN mkdir -p /workspace /home/node/.claude && \
  chown -R node:node /workspace /home/node/.claude

WORKDIR /workspace

ARG GIT_DELTA_VERSION=0.18.2
RUN ARCH=$(dpkg --print-architecture) && \
  wget "https://github.com/dandavison/delta/releases/download/${GIT_DELTA_VERSION}/git-delta_${GIT_DELTA_VERSION}_${ARCH}.deb" && \
  sudo dpkg -i "git-delta_${GIT_DELTA_VERSION}_${ARCH}.deb" && \
  rm "git-delta_${GIT_DELTA_VERSION}_${ARCH}.deb"

# Set up non-root user
USER node

# Install global packages
ENV NPM_CONFIG_PREFIX=/usr/local/share/npm-global
ENV PATH=$PATH:/usr/local/share/npm-global/bin

# Set the default shell to zsh rather than sh
ENV SHELL=/bin/zsh

# Set the default editor and visual
ENV EDITOR=nano
ENV VISUAL=nano

# Default powerline10k theme
ARG ZSH_IN_DOCKER_VERSION=1.2.0
RUN sh -c "$(wget -O- https://github.com/deluan/zsh-in-docker/releases/download/v${ZSH_IN_DOCKER_VERSION}/zsh-in-docker.sh)" -- \
  -p git \
  -p fzf \
  -a "source /usr/share/doc/fzf/examples/key-bindings.zsh" \
  -a "source /usr/share/doc/fzf/examples/completion.zsh" \
  -a "export PROMPT_COMMAND='history -a' && export HISTFILE=/commandhistory/.bash_history" \
  -a "if ! pgrep -f 'socat.*9222' > /dev/null; then /usr/local/bin/start-chrome-proxy.sh; fi" \
  -a "alias claude='claude --dangerously-skip-permissions'" \
  -x

# Install Claude Code
RUN npm install -g @anthropic-ai/claude-code@${CLAUDE_CODE_VERSION}

# Install Rust toolchain
USER root
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile default
ENV PATH="/root/.cargo/bin:${PATH}"

# Install WASM target and tools as root, then copy to node user
RUN /root/.cargo/bin/rustup target add wasm32-unknown-unknown && \
  /root/.cargo/bin/cargo install wasm-pack trunk

# Copy Rust installation to node user
RUN cp -r /root/.cargo /home/node/.cargo && \
  cp -r /root/.rustup /home/node/.rustup && \
  chown -R node:node /home/node/.cargo /home/node/.rustup

USER node
ENV PATH="/home/node/.cargo/bin:${PATH}"

# Create Chrome proxy startup script
# This forwards port 9222 from the host to the container
USER root
RUN echo '#!/bin/bash' > /usr/local/bin/start-chrome-proxy.sh && \
  echo 'nohup socat TCP-LISTEN:9222,bind=127.0.0.1,fork,reuseaddr TCP:host.docker.internal:9222 </dev/null >/tmp/chrome-proxy.log 2>&1 &' >> /usr/local/bin/start-chrome-proxy.sh && \
  chmod +x /usr/local/bin/start-chrome-proxy.sh
USER node

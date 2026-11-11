# Image to build the songs in (CI of the songbook repo, tests and publish of this one, `make DOCKER=1` of the songbook
# repo): everything `band-songbook` needs, and `band-songbook` itself.
#
# The same Ubuntu release and packages as the dev machine (Ubuntu 24.04: TeX Live 2023, LilyPond 2.24.3), so that the CI
# draws the sheets the way they come out locally. The tool needs fluidsynth, a soundfont and ffmpeg for the mp3 renders
# (src/nodes/mp3render.rs), lualatex with the packages of the templates (verse and nicematrix are not in TeX Live 2023
# as the dev machine has them: vendored in docker/build/texmf), lilypond, and the chord fonts of band-songbook/fonts.
#
# Built from the root of the repo by .github/workflows/build-image.yml:
#   docker build -f docker/build/Dockerfile -t songbook-build .
# Not the image of the Lambda (band-songbook/Dockerfile): that one is on the Amazon Lambda base and has no fluidsynth.

# ---- the tool -------------------------------------------------------------------------------------------------------
FROM ubuntu:24.04 AS builder

ENV DEBIAN_FRONTEND=noninteractive

ARG UBUNTU_MIRROR=
RUN if [ -n "$UBUNTU_MIRROR" ]; then \
        sed -i "s|http://archive.ubuntu.com/ubuntu|$UBUNTU_MIRROR|g; s|http://security.ubuntu.com/ubuntu|$UBUNTU_MIRROR|g" \
            /etc/apt/sources.list.d/ubuntu.sources; \
    fi
RUN apt-get update && apt-get install -y build-essential pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Rust from the official image (same version as the Lambda image), not `curl | sh`: a failed download of a piped script
# does not fail the build.
COPY --from=rust:1.88-bookworm /usr/local/rustup /usr/local/rustup
COPY --from=rust:1.88-bookworm /usr/local/cargo /usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo PATH=/usr/local/cargo/bin:$PATH

WORKDIR /usr/src/app
COPY band-songbook/Cargo.toml ./
COPY band-songbook/src ./src
# Cargo.lock is not committed: the dependencies resolve to the latest ones that build with this Rust.
RUN cargo build --release --bin band-songbook

# ---- the image ------------------------------------------------------------------------------------------------------
FROM ubuntu:24.04

LABEL org.opencontainers.image.source="https://github.com/laurentcarrie/songbook-lambda"
LABEL org.opencontainers.image.description="Build image of the songbook: band-songbook, texlive, lilypond, fluidsynth, ffmpeg, fonts, aws, rust"

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

# Optional apt mirror (the same for the main and the security pockets): the GitHub runners (ubuntu-26.04) resolve neither
# archive.ubuntu.com, security.ubuntu.com nor azure.archive.ubuntu.com, in the build containers and on the host.
# mirrors.kernel.org resolves. Empty by default (the Ubuntu ones).
ARG UBUNTU_MIRROR=
RUN if [ -n "$UBUNTU_MIRROR" ]; then \
        sed -i "s|http://archive.ubuntu.com/ubuntu|$UBUNTU_MIRROR|g; s|http://security.ubuntu.com/ubuntu|$UBUNTU_MIRROR|g" \
            /etc/apt/sources.list.d/ubuntu.sources; \
    fi

# Same TeX Live packages as the dev machine (the recommended ones are installed too, as there).
RUN apt-get update && apt-get install -y \
        texlive-base texlive-binaries texlive-extra-utils \
        texlive-fonts-extra texlive-fonts-extra-links texlive-fonts-recommended \
        texlive-latex-base texlive-latex-extra texlive-latex-recommended \
        texlive-luatex texlive-pictures texlive-plain-generic \
    && rm -rf /var/lib/apt/lists/*

# LilyPond, the mp3 renders (fluidsynth + soundfont + ffmpeg), the system fonts the sheets can ask for, and the tools of
# the Makefile of the songbook repo and of the CI jobs.
RUN apt-get update && apt-get install -y \
        lilypond fluidsynth fluid-soundfont-gm ffmpeg \
        fonts-adf-accanthis fonts-adf-berenis fonts-adf-gillius fonts-adf-universalis fonts-cabin fonts-cantarell \
        fonts-clear-sans fonts-comfortaa fonts-comic-neue fonts-croscore fonts-crosextra-caladea \
        fonts-crosextra-carlito fonts-droid-fallback fonts-ebgaramond-extra fonts-font-awesome fonts-freefont-otf \
        fonts-freefont-ttf fonts-gfs-artemisia fonts-gfs-complutum fonts-gfs-didot fonts-gfs-neohellenic \
        fonts-gfs-olga fonts-gfs-solomos fonts-go fonts-inter fonts-lato fonts-linuxlibertine fonts-lmodern \
        fonts-lobster fonts-lobstertwo fonts-oflb-asana-math fonts-open-sans fonts-opensymbol fonts-paratype \
        fonts-roboto-slab fonts-roboto-unhinted fonts-sil-andika fonts-sil-charis fonts-sil-gentium \
        fonts-sil-gentium-basic fonts-sil-gentiumplus fonts-sil-gentiumplus-compact fonts-stix fonts-texgyre \
        fonts-texgyre-math fonts-tuffy fonts-ubuntu fonts-urw-base35 fontconfig \
        make git curl ca-certificates unzip python3 tar gzip \
        build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# AWS CLI v2 (not in the Ubuntu 24.04 repositories).
RUN curl -fsSL https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip -o /tmp/awscliv2.zip \
    && unzip -q /tmp/awscliv2.zip -d /tmp && /tmp/aws/install && rm -rf /tmp/aws /tmp/awscliv2.zip

# Rust (tests and publish of this repo run in the image), same layout as the official rust image.
COPY --from=rust:1.88-bookworm /usr/local/rustup /usr/local/rustup
COPY --from=rust:1.88-bookworm /usr/local/cargo /usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo PATH=/usr/local/cargo/bin:$PATH

# Two LaTeX packages the templates load that no Ubuntu package provides, here the very files of the dev machine
# (~/texmf): verse.sty, and nicematrix.sty (newer than the one of TeX Live 2023). In TEXMFLOCAL.
COPY docker/build/texmf /usr/local/share/texmf
RUN mktexlsr /usr/local/share/texmf

# The chord fonts (the empty placeholders of the folder are not fonts).
COPY band-songbook/fonts /usr/share/fonts/custom
RUN find /usr/share/fonts/custom -type f -size 0 -delete && fc-cache -f

# The tool.
COPY --from=builder /usr/src/app/target/release/band-songbook /usr/local/bin/band-songbook

# Fails the build of the image, not a job, when something is missing.
RUN set -e; \
    for t in band-songbook lilypond lualatex fluidsynth ffmpeg aws cargo make python3 curl; do command -v "$t" >/dev/null || { echo "missing: $t"; exit 1; }; done; \
    test -f /usr/share/sounds/sf2/FluidR3_GM.sf2; \
    kpsewhich verse.sty nicematrix.sty | grep -c . | grep -qx 2; \
    test "$(fc-list | grep -ci songbook)" -ge 5; \
    ffmpeg -hide_banner -encoders 2>/dev/null | grep -q libmp3lame; \
    band-songbook --help >/dev/null; \
    lilypond --version | head -1; lualatex --version | head -1; aws --version; cargo --version

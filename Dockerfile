FROM rust:1-trixie@sha256:09027f873d45b8ecd1dc86f1fa13b7a62b875c961f017a0e331267b350430137

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        gcc-mingw-w64-x86-64 wine wine64 xvfb xauth \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-pc-windows-gnu \
    && rustup component add rustfmt clippy

ENV CARGO_BUILD_TARGET=x86_64-pc-windows-gnu \
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER=wine-test \
    CARGO_TARGET_DIR=/build \
    WINEPREFIX=/opt/wine \
    WINEARCH=win64 \
    WINEDEBUG=-all \
    RUSTFLAGS="--remap-path-prefix=/workspace=/src --remap-path-prefix=/usr/local/cargo=/cargo"

COPY docker/wine-test /usr/local/bin/wine-test
RUN chmod +x /usr/local/bin/wine-test \
    && xvfb-run -a wineboot --init \
    && wineserver -w

WORKDIR /workspace
CMD ["sleep", "infinity"]

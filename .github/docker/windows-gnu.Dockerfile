# Tools for the "Windows zigbuild + Wine" CI jobs, which cross-compile the
# x86_64-pc-windows-gnu tests on the runner and run them under Wine:
# - zig at /opt/zig, which the jobs copy out of the image to build with;
# - Wine, which the jobs run the test executables in.
# The windows-gnu-image job of the Validation workflow publishes it to ghcr.io,
# tagged with a hash of this file (see windows-gnu-image-ref.sh), whenever that
# tag is not published yet.
FROM ubuntu:26.04 AS zig

ARG ZIG_VERSION=0.14.1
ARG ZIG_SHA256=24aeeec8af16c381934a6cd7d95c807a8cb2cf7df9fa40d359aa884195c4716c
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install --yes --no-install-recommends \
        ca-certificates curl xz-utils \
    && curl --fail --location --silent --show-error --output /tmp/zig.tar.xz \
        "https://ziglang.org/download/${ZIG_VERSION}/zig-x86_64-linux-${ZIG_VERSION}.tar.xz" \
    && echo "${ZIG_SHA256}  /tmp/zig.tar.xz" | sha256sum --check \
    && mkdir /opt/zig \
    && tar --extract --file /tmp/zig.tar.xz --directory /opt/zig --strip-components=1

FROM ubuntu:26.04

# The test executables are 64-bit only, so drop Wine's 32-bit Windows DLLs
# (~560 MB). The Wine prefix is deliberately not created here: Ubuntu's Wine
# copies its DLLs into it, which would add another ~1.2 GB to the image, while
# creating it at run time only takes a couple of seconds. Wine only accepts a
# prefix owned by the user running it, so point WINEPREFIX at a directory made
# by that user, e.g. `export WINEPREFIX="$(mktemp -d)"`.
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install --yes --no-install-recommends wine wine64 \
    && rm -rf /var/lib/apt/lists/* /usr/lib/x86_64-linux-gnu/wine/i386-windows

COPY --from=zig /opt/zig /opt/zig

ENV WINEDEBUG=-all

LABEL org.opencontainers.image.source=https://github.com/DataDog/libddwaf-rust \
      org.opencontainers.image.description="zig and Wine for libddwaf-rust Windows CI tests"

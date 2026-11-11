# Wine runtime for running the x86_64-pc-windows-gnu test executables that the
# "Windows zigbuild + Wine" CI jobs cross-compile on the runner. It is published
# to ghcr.io by .github/workflows/ci-image.yml, tagged with a hash of this file
# (see wine-image-ref.sh); jobs build it locally if that tag is not published.
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

ENV WINEDEBUG=-all

LABEL org.opencontainers.image.source=https://github.com/DataDog/libddwaf-rust \
      org.opencontainers.image.description="Wine runtime for libddwaf-rust Windows CI tests"

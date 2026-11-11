#!/bin/sh
# Prints the ghcr.io reference of the Wine CI image. The tag is derived from the
# Dockerfile's contents, so changing it yields a new tag rather than silently
# replacing an image that existing commits rely on.
set -eu
dir=$(dirname "$0")
hash=$(sha256sum "$dir/wine.Dockerfile" | cut -c1-16)
echo "ghcr.io/datadog/libddwaf-rust-ci:wine-$hash"

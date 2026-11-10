#!/bin/sh
set -eu
# The native AVIF build invokes CMake directly, outside the cmake Rust crate.
if [ "${HIRUNE_CROSS_ARCH:-}" = arm64 ] && [ "${1:-}" = -S ]; then
  exec /usr/bin/cmake "$@" -DCMAKE_TOOLCHAIN_FILE=/opt/toolchains/arm64.cmake
fi
exec /usr/bin/cmake "$@"

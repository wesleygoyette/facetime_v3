#!/bin/bash

set -euo pipefail

echo "Checking for Homebrew..."
if ! command -v brew &> /dev/null; then
  echo "Error: Homebrew is not installed. Please install it first." >&2
  exit 1
fi

# Check and install LLVM if missing
if ! brew list llvm &> /dev/null; then
  echo "LLVM not found. Installing LLVM..."
  brew install llvm
else
  echo "LLVM found."
fi

# Check and install OpenCV if missing
if ! brew list opencv &> /dev/null; then
  echo "OpenCV not found. Installing OpenCV..."
  brew install opencv
else
  echo "OpenCV found."
fi

LLVM_PREFIX="$(brew --prefix llvm)"
OPENCV_PREFIX="$(brew --prefix opencv)"

# Optional: Uncomment if having issues with dylibs
# export CPATH="$LLVM_PREFIX/include/c++/v1"
export LIBCLANG_PATH="$LLVM_PREFIX/lib"
export DYLD_LIBRARY_PATH="$LLVM_PREFIX/lib:$OPENCV_PREFIX/lib"

echo "Environment configured."
echo "Building the client in release mode..."
cargo build --bin client --release
echo "Build complete."

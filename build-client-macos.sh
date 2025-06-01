#!/bin/bash

set -euo pipefail

echo "Checking for Homebrew..."
if ! command -v brew &> /dev/null; then
  echo "Error: Homebrew is not installed. Please install it first." >&2
  exit 1
fi

if ! brew list llvm &> /dev/null; then
  echo "LLVM not found. Installing LLVM..."
  brew install llvm
else
  echo "LLVM found."
fi

if ! brew list opencv &> /dev/null; then
  echo "OpenCV not found. Installing OpenCV..."
  brew install opencv
else
  echo "OpenCV found."
fi

LLVM_PREFIX="$(brew --prefix llvm)"
OPENCV_PREFIX="$(brew --prefix opencv)"

if ! printf '#include <memory>\nint main() { return 0; }' | clang++ -x c++ - -o /dev/null 2>/dev/null; then
  echo "C++ standard headers not found. Setting CPATH..."
  export CPATH="$LLVM_PREFIX/include/c++/v1"
else
  echo "C++ standard headers found. CPATH not needed."
fi

export LIBCLANG_PATH="$LLVM_PREFIX/lib"
export DYLD_LIBRARY_PATH="$LLVM_PREFIX/lib:$OPENCV_PREFIX/lib"

echo "Environment configured."
echo "Building the client in release mode..."
cargo build --bin client --release
echo "Build complete."
echo "You can now run the client with:"
echo "./target/release/client"

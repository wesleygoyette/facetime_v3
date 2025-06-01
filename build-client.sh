#!/bin/bash

export CPATH="$(brew --prefix llvm)/include/c++/v1"
export LIBCLANG_PATH="$(brew --prefix llvm)/lib"
export DYLD_LIBRARY_PATH="$(brew --prefix llvm)/lib"
cargo build --bin client -r


#!/bin/bash
cargo build --release
if [ $? -eq 0 ]; then
    echo "Binario: target/release/raecli2"
    ls -lh target/release/raecli2
else
    exit 1
fi

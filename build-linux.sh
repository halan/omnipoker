#!/bin/bash

echo "Building the frontend...🚀"
cd frontend
trunk build --release
cd ..

# Build Linux x86_64 using Docker
echo "Building the backend...🚀"
docker run --rm -v "$(pwd)":/usr/src/myapp -w /usr/src/myapp --platform linux/amd64 rust:latest \
    cargo build --release --target x86_64-unknown-linux-gnu
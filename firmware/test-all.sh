#!/bin/bash

# Test nanofish on host target to avoid no_std conflicts
echo "Testing nanofish on host target..."
cargo test -p nanofish --target x86_64-apple-darwin

echo "Testing leasehund on host target..."
cargo test -p leasehund --target x86_64-apple-darwin

echo "Testing common on host target..."
cargo test -p common --target x86_64-apple-darwin

echo "All tests completed!"

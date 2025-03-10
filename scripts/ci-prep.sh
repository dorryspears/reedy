#!/bin/bash
set -e

echo "Running rustfmt to fix formatting issues..."
cargo fmt

echo "Running clippy --fix to auto-fix Clippy warnings..."
cargo clippy --fix --allow-dirty --allow-staged

echo "CI preparation complete!" 
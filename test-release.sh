#!/bin/bash
set -e

echo "🚀 Testing release build process locally"

# Clean any previous builds
echo "🧹 Cleaning previous builds"
cargo clean

# Install dependencies if needed (macOS specific)
echo "📦 Installing dependencies"
if [[ "$OSTYPE" == "darwin"* ]]; then
  echo "Installing macOS dependencies"
  brew install openssl
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
  echo "Installing Linux dependencies"
  sudo apt-get update
  sudo apt-get install -y libssl-dev pkg-config
fi

# Run CI preparation script if it exists
if [ -f "scripts/ci-prep.sh" ]; then
  echo "🔧 Running CI preparation script"
  chmod +x scripts/ci-prep.sh
  ./scripts/ci-prep.sh
fi

# Build release version
echo "🏗️ Building release binary"
cargo build --release

# Test the binary
echo "🧪 Testing binary"
if [ -f "target/release/reedy" ]; then
  echo "Binary exists at target/release/reedy"
  file target/release/reedy
  ls -la target/release/reedy
else
  echo "⚠️ Binary not found!"
  exit 1
fi

echo "✅ Local release build test completed successfully!" 
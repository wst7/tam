#!/bin/bash
set -euo pipefail

echo "🚀 Starting release process..."
if cargo release patch --no-verify --no-publish --execute; then
    echo "✅ Release successful"
else
    echo "❌ Release failed"
    exit 1
fi

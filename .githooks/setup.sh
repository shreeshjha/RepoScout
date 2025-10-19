#!/bin/bash
# Setup script to install git hooks

set -e

echo "Setting up git hooks..."

# Configure git to use our hooks directory
git config core.hooksPath .githooks

echo "✅ Git hooks installed successfully!"
echo "Run 'git config --unset core.hooksPath' to disable them."

#!/usr/bin/env bash
# Release helper script for grok-cli
# Author: John McConnell john.microtech@gmail.com
# Repository: https://github.com/microtech/grok-cli

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Functions
print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Check if we're in the project root
if [ ! -f "Cargo.toml" ]; then
    print_error "Must be run from the project root directory"
    exit 1
fi

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
print_info "Current version in Cargo.toml: $CURRENT_VERSION"

# Check if version is provided as argument
if [ -z "$1" ]; then
    print_info "Usage: ./scripts/release.sh <version>"
    print_info "Example: ./scripts/release.sh 0.1.4"
    echo ""
    print_warning "No version specified. Using current version: v$CURRENT_VERSION"
    VERSION="$CURRENT_VERSION"
else
    VERSION="$1"
    # Remove 'v' prefix if provided
    VERSION="${VERSION#v}"
fi

TAG_NAME="v$VERSION"

print_info "Preparing release: $TAG_NAME"
echo ""

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    print_error "You have uncommitted changes. Please commit or stash them first."
    git status --short
    exit 1
fi

# Check if we're on the right branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
print_info "Current branch: $CURRENT_BRANCH"

if [ "$CURRENT_BRANCH" != "master" ] && [ "$CURRENT_BRANCH" != "main" ]; then
    print_warning "You're not on master/main branch. Continue? (y/N)"
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        print_info "Release cancelled"
        exit 0
    fi
fi

# Check if tag already exists locally
if git rev-parse "$TAG_NAME" >/dev/null 2>&1; then
    print_warning "Tag $TAG_NAME already exists locally"
    print_info "Delete and recreate? (y/N)"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        git tag -d "$TAG_NAME"
        print_success "Local tag deleted"
    else
        print_error "Cannot continue with existing tag"
        exit 1
    fi
fi

# Check if tag exists on remote
if git ls-remote --tags origin | grep -q "refs/tags/$TAG_NAME"; then
    print_warning "Tag $TAG_NAME already exists on remote"
    print_info "Delete remote tag and recreate? (y/N)"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        git push origin ":refs/tags/$TAG_NAME"
        print_success "Remote tag deleted"
    else
        print_error "Cannot continue with existing tag"
        exit 1
    fi
fi

# Update version in Cargo.toml if different
if [ "$VERSION" != "$CURRENT_VERSION" ]; then
    print_info "Updating Cargo.toml version to $VERSION"
    sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
    rm Cargo.toml.bak 2>/dev/null || true

    # Commit the version change
    git add Cargo.toml
    git commit -m "Bump version to $VERSION"
    print_success "Version updated and committed"
fi

# Update CHANGELOG.md reminder
print_warning "Have you updated CHANGELOG.md for this release? (y/N)"
read -r response
if [[ ! "$response" =~ ^[Yy]$ ]]; then
    print_info "Please update CHANGELOG.md and run this script again"
    exit 0
fi

# Run tests
print_info "Running tests..."
if cargo test --quiet; then
    print_success "Tests passed"
else
    print_error "Tests failed. Fix tests before releasing."
    exit 1
fi

# Build release binaries locally to verify
print_info "Building release binary locally..."
if cargo build --release --quiet; then
    print_success "Local build successful"
else
    print_error "Build failed. Fix build errors before releasing."
    exit 1
fi

# Create the tag
print_info "Creating tag $TAG_NAME"
git tag -a "$TAG_NAME" -m "Release $TAG_NAME"
print_success "Tag created"

# Push changes
print_info "Pushing changes to origin..."
git push origin "$CURRENT_BRANCH"
print_success "Changes pushed"

# Push tag
print_info "Pushing tag to origin..."
git push origin "$TAG_NAME"
print_success "Tag pushed"

echo ""
print_success "Release $TAG_NAME initiated!"
echo ""
print_info "GitHub Actions will now build binaries for:"
echo "  - Linux (x86_64)"
echo "  - macOS (x86_64)"
echo "  - Windows (x86_64)"
echo ""
print_info "Check the GitHub Actions workflow at:"
echo "  https://github.com/microtech/grok-cli/actions"
echo ""
print_info "Once complete, the release will be available at:"
echo "  https://github.com/microtech/grok-cli/releases/tag/$TAG_NAME"
echo ""
print_success "Done! 🚀"

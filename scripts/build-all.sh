#!/bin/bash

# Build verification script for gc9d01-rs project
# This script builds the main library and all example projects to ensure they compile correctly

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to build a project
build_project() {
    local project_path="$1"
    local project_name="$2"
    local features="$3"
    
    print_status "Building $project_name..."
    
    cd "$project_path"
    
    if [ -n "$features" ]; then
        print_status "  Features: $features"
        if cargo build --features "$features"; then
            print_success "  ✓ $project_name built successfully with features: $features"
        else
            print_error "  ✗ Failed to build $project_name with features: $features"
            return 1
        fi
    else
        if cargo build; then
            print_success "  ✓ $project_name built successfully"
        else
            print_error "  ✗ Failed to build $project_name"
            return 1
        fi
    fi
    
    cd - > /dev/null
}

# Function to run tests for a project
test_project() {
    local project_path="$1"
    local project_name="$2"
    local features="$3"
    
    print_status "Testing $project_name..."
    
    cd "$project_path"
    
    if [ -n "$features" ]; then
        print_status "  Features: $features"
        if cargo test --features "$features"; then
            print_success "  ✓ $project_name tests passed with features: $features"
        else
            print_error "  ✗ Tests failed for $project_name with features: $features"
            return 1
        fi
    else
        if cargo test; then
            print_success "  ✓ $project_name tests passed"
        else
            print_error "  ✗ Tests failed for $project_name"
            return 1
        fi
    fi
    
    cd - > /dev/null
}

# Get the script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

print_status "Starting build verification for gc9d01-rs project"
print_status "Project root: $PROJECT_ROOT"

cd "$PROJECT_ROOT"

# Track build results
FAILED_BUILDS=()
SUCCESSFUL_BUILDS=()

# Build main library with different feature combinations
print_status "=== Building main library ==="

# Build without features
if build_project "$PROJECT_ROOT" "gc9d01 (no features)" ""; then
    SUCCESSFUL_BUILDS+=("gc9d01 (no features)")
else
    FAILED_BUILDS+=("gc9d01 (no features)")
fi

# Build with async feature
if build_project "$PROJECT_ROOT" "gc9d01 (async)" "async"; then
    SUCCESSFUL_BUILDS+=("gc9d01 (async)")
else
    FAILED_BUILDS+=("gc9d01 (async)")
fi

# Build with defmt feature
if build_project "$PROJECT_ROOT" "gc9d01 (defmt)" "defmt"; then
    SUCCESSFUL_BUILDS+=("gc9d01 (defmt)")
else
    FAILED_BUILDS+=("gc9d01 (defmt)")
fi

# Build with both features
if build_project "$PROJECT_ROOT" "gc9d01 (async,defmt)" "async,defmt"; then
    SUCCESSFUL_BUILDS+=("gc9d01 (async,defmt)")
else
    FAILED_BUILDS+=("gc9d01 (async,defmt)")
fi

# Run tests for main library
print_status "=== Testing main library ==="

if test_project "$PROJECT_ROOT" "gc9d01 tests" ""; then
    SUCCESSFUL_BUILDS+=("gc9d01 tests")
else
    FAILED_BUILDS+=("gc9d01 tests")
fi

# Build all example projects
print_status "=== Building example projects ==="

# List of example projects
EXAMPLES=(
    "examples/stm32g4-160-40"
    "examples/stm32g4-160-40-90-complex-patterns"
    "examples/stm32g4-160-40-direct-spi"
    "examples/stm32g4-160-40-direct-spi-90-complex-patterns"
)

for example in "${EXAMPLES[@]}"; do
    example_path="$PROJECT_ROOT/$example"
    example_name=$(basename "$example")
    
    if [ -d "$example_path" ]; then
        if build_project "$example_path" "$example_name" ""; then
            SUCCESSFUL_BUILDS+=("$example_name")
        else
            FAILED_BUILDS+=("$example_name")
        fi
    else
        print_warning "Example directory not found: $example_path"
        FAILED_BUILDS+=("$example_name (not found)")
    fi
done

# Print summary
print_status "=== Build Summary ==="

if [ ${#SUCCESSFUL_BUILDS[@]} -gt 0 ]; then
    print_success "Successful builds (${#SUCCESSFUL_BUILDS[@]}):"
    for build in "${SUCCESSFUL_BUILDS[@]}"; do
        echo -e "  ${GREEN}✓${NC} $build"
    done
fi

if [ ${#FAILED_BUILDS[@]} -gt 0 ]; then
    print_error "Failed builds (${#FAILED_BUILDS[@]}):"
    for build in "${FAILED_BUILDS[@]}"; do
        echo -e "  ${RED}✗${NC} $build"
    done
    
    print_error "Build verification failed!"
    exit 1
else
    print_success "All builds completed successfully!"
    exit 0
fi

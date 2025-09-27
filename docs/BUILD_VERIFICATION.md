# Build Verification System

This document describes the build verification system for the gc9d01-rs project, which ensures all test cases and example projects build correctly.

## Overview

The build verification system consists of:

1. **Build verification script** (`scripts/build-all.sh`)
2. **Pre-commit hooks** (configured in `lefthook.yml`)
3. **CI/CD pipeline** (configured in `.github/workflows/build.yml`)

## Components

### 1. Build Verification Script

**Location**: `scripts/build-all.sh`

This script automatically builds and tests:
- Main library with different feature combinations:
  - No features
  - `async` feature only
  - `defmt` feature only
  - Both `async` and `defmt` features
- All example projects:
  - `examples/stm32g4-160-40`
  - `examples/stm32g4-160-40-90-complex-patterns`
  - `examples/stm32g4-160-40-direct-spi`
  - `examples/stm32g4-160-40-direct-spi-90-complex-patterns`

**Usage**:
```bash
# Run from project root
./scripts/build-all.sh
```

**Features**:
- Colored output for easy reading
- Detailed build status reporting
- Exit code 0 on success, 1 on failure
- Summary of successful and failed builds

### 2. Pre-commit Hooks

**Location**: `lefthook.yml`

The pre-commit hooks run automatically before each commit and include:
- **Code formatting** (`cargo fmt`)
- **Linting** (`cargo clippy --all-targets --all-features`)
- **Build verification** (runs `scripts/build-all.sh`)

**Setup**:
```bash
# Install lefthook (if not already installed)
# On macOS: brew install lefthook
# On other systems: see https://github.com/evilmartians/lefthook

# Install hooks
lefthook install
```

**Configuration**:
- Hooks run sequentially (not in parallel) to ensure proper order
- Build verification is skipped during merge and rebase operations
- Code formatting automatically stages fixed files

### 3. CI/CD Pipeline

**Location**: `.github/workflows/build.yml`

The GitHub Actions workflow includes three jobs:

#### Job 1: Library Build and Test
- Tests the main library with all feature combinations
- Runs formatting checks and clippy linting
- Uses matrix strategy for parallel testing of different features

#### Job 2: Example Projects Build
- Builds all example projects individually
- Uses matrix strategy for parallel building
- Installs required ARM target (`thumbv7em-none-eabihf`)

#### Job 3: Comprehensive Build Verification
- Runs the complete build verification script
- Depends on successful completion of the first two jobs
- Provides final verification that everything works together

## Supported Targets

The build verification system supports:
- **Host target**: `x86_64-unknown-linux-gnu` (for library and tests)
- **Embedded target**: `thumbv7em-none-eabihf` (for STM32G4 examples)

## Feature Combinations Tested

The system tests all possible combinations of optional features:
- Base library (no optional features)
- `async` - Enables async/await support
- `defmt` - Enables defmt logging support
- `async,defmt` - Both features enabled

## Error Handling

The build verification script:
- Stops on first error when running individual builds
- Continues through all projects to provide complete status
- Returns appropriate exit codes for CI/CD integration
- Provides detailed error messages with colored output

## Maintenance

### Adding New Example Projects

1. Add the new example directory to the `EXAMPLES` array in `scripts/build-all.sh`
2. Add the new example to the matrix in `.github/workflows/build.yml`
3. Test the changes by running `./scripts/build-all.sh`

### Adding New Features

1. Update the feature combinations in `scripts/build-all.sh`
2. Update the matrix strategy in `.github/workflows/build.yml`
3. Test all combinations to ensure they build correctly

## Troubleshooting

### Common Issues

1. **Missing ARM target**: Install with `rustup target add thumbv7em-none-eabihf`
2. **Lefthook not installed**: Install using your package manager or from GitHub releases
3. **Build failures**: Check individual project dependencies and Rust version compatibility

### Debug Mode

To see more detailed output from the build script:
```bash
# Run with verbose cargo output
CARGO_TERM_COLOR=always ./scripts/build-all.sh
```

## Integration with Development Workflow

1. **Local Development**: Pre-commit hooks ensure code quality before commits
2. **Pull Requests**: CI pipeline validates all changes automatically
3. **Release Process**: Build verification ensures all examples work with new versions

This system ensures that all test cases and example projects remain buildable throughout the development lifecycle.

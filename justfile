alias b := build
alias c := check
alias d := delete
alias f := fmt
alias r := run
alias t := test

_default:
    @just --list

# Build
build:
    cargo build

# Check code: formatting, compilation, linting, and commit signature
check:
    cargo +nightly fmt --all -- --check
    cargo check
    cargo clippy --workspace --all-targets -- -D warnings

# Delete files: example, target, lockfile
delete item="examples":
    just _delete-{{ item }}

# Format code
fmt:
    cargo +nightly fmt

# Run a ROM in the SDL2 frontend
run rom="Tetris [Fran Dachille, 1991].ch8":
    cargo run -p emuchip-8-sdl -- "{{ rom }}"

test:
    cargo test --workspace

_delete-target:
    rm -rf target/

_delete-lockfile:
    rm -f Cargo.lock
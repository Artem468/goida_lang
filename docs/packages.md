# Goida package and build standard

Every project and reusable package is described by `goida.toml`. Dependencies
are installed into the active `GOIDA_VENV`, or into the project's `.goida`
environment when no environment is active.

## Commands

- `goida venv` creates the local package environment.
- `goida add <name> --git <url>` or `--path <path>` adds and synchronizes a dependency.
- `goida sync` installs all direct and transitive dependencies and rewrites `goida.lock`.
- `goida build` runs `goida sync`, then executes the root package build contract.
- `goida remove <name>` removes a direct dependency.

`goida.lock` records resolved sources, Git revisions, install paths and native
artifacts. Commit it for applications. Libraries may omit it when consumers
must resolve their own dependency graph.

## Basic manifest

```toml
[package]
name = "example"
description = "Example package"
version = "0.1.0"
entry = "main.goida"

[dependencies.text-utils]
git = "https://example.org/text-utils.git"
tag = "v1.2.0"

[dependencies.local-helper]
path = "vendor/local-helper"
```

Dependency names share one flat import namespace. Resolving the same name from
different sources is an error.

## Native build contract

A package that provides `.dll`, `.so` or `.dylib` files declares how they are
produced and where they are installed:

```toml
[build]
command = ["cargo", "build", "--release"]
workdir = "."

[[build.artifacts]]
source = "target/release/example.dll"
destination = "native/example.dll"
platforms = ["windows"]

[[build.artifacts]]
source = "target/release/libexample.so"
destination = "native/libexample.so"
platforms = ["linux-x86_64"]

[[build.artifacts]]
source = "target/release/libexample.dylib"
destination = "native/libexample.dylib"
platforms = ["macos"]
```

`command` is an argument array, not a shell string. The first item is the
program. Packages can use Cargo, CMake, Meson or another build tool without
hardcoded package-manager behavior.

The build process receives:

- `GOIDA_PACKAGE_ROOT`: absolute package root.
- `GOIDA_TARGET_PLATFORM`: current `<os>-<arch>` selector.

Supported artifact selectors are `*`, an OS such as `windows` or `linux`, and
an exact selector such as `windows-x86_64`.

All artifact paths must remain inside the package. After the command succeeds,
every artifact selected for the current platform must exist. It is copied to
its `destination`, which should be the stable path referenced by Goida source:

```goida
library "native/example" {
    function calculate(value: number): number {}
}
```

## Prebuilt native packages

To distribute already compiled libraries, omit `build.command` and commit the
platform artifacts:

```toml
[[build.artifacts]]
source = "prebuilt/windows-x86_64/example.dll"
destination = "native/example.dll"
platforms = ["windows-x86_64"]

[[build.artifacts]]
source = "prebuilt/linux-x86_64/libexample.so"
destination = "native/libexample.so"
platforms = ["linux-x86_64"]
```

Installation fails when a required prebuilt artifact is missing. Build commands
execute code from dependencies and must therefore only be accepted from trusted
sources.

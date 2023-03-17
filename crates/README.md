# Crate index

This file is an index of everything under `crates/`, with a brief description
of what to do. Please keep it up to date as crates are removed/added, with
entries in alphabetical order. For more details on individual crates, check
their own READMEs and rustdocs.

## Crates

### [nassun](./nassun)

Package API for resolving and interacting with individual packages. This
exposes functionality to fetch metadata for, resolve (from specifiers like
`foo@^1`), and download packages, among other things, for all supported
package types (NPM registry, git, directories, etc).

### [node-maintainer](./node-maintainer)

Dependency tree resolver. Wraps Nassun and exposes functionality to resolve
entire project trees, generate lockfiles, and extract everything to their
final destination in `node_modules`.

### [oro-client](./oro-client)

NPM registry client for interacting with HTTP APIs compatible with
`https://registry.npmjs.org`. This includes basics like package metadata and
tarball downloads, but includes API endpoints beyond that, like the `ping`
endpoint. This abstracts over [`reqwest`](https://docs.rs/reqwest).

### [oro-common](./oro-common)

Common types and utilities used across the orogene project. As of this
writing, this was mostly just manifest, metadata, and packument types, along
with all their serde-related implementation details.

### [oro-config](./oro-config)

Functionality for reading and managing orogene config files.

### [oro-config-derive](./oro-config-derive)

Implements a derive macro used by the orogene CLI to "intelligently" layer CLI
arguments with configuration values from config files, such that orogene CLI
SubCommand structs can have their fields filled in by config file values and
other defaults when command line arguments weren't passed in for them.

### [oro-package-spec](./oro-package-spec)

Parser for package specifiers. That is, expressions like `foo@^1.2.3` or
`bar@npm:some-alias`.

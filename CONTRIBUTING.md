# Contributing to orogene

## Finding Something to Do

Orogene [has a roadmap](https://github.com/orgs/orogene/projects/2/views/1)
and welcomes contributions! Issues looking for outside help are tagged as
[`help wanted`](https://github.com/orogene/orogene/labels/help%20wanted). On
top of that, there are [`good first
issue`](https://github.com/orogene/orogene/labels/good%20first%20issue) for
those looking to get started more entry-level tasks.

## Coding Conventions

Orogene uses [conventional
commits](https://www.conventionalcommits.org/en/v1.0.0/) as its commit message
format. Ideally, any pull requests will already use this style when
submitting, but commit messages will simply be editorialized on merge if not,
so consider this a soft request.

This repo uses both `clippy` and `rustfmt` to maintain consistency. Before
submitting a pull request, please make sure to run both `cargo clippy --all`
and `cargo fmt --all`.

## Getting up and running

Orogene is a pretty run-of-the-mill Rust app and only requires a couple of
steps to get up and running.

### Build Dependencies

You will need [git](https://git-scm.com/downloads) in order to fetch the
orogene sources. Next, to get a checkout:

```
git clone https://github.com/orogene/orogene.git
cd orogene
```

Additionally, some tools are required to compile/build orogene:

1. (Linux only) An available OpenSSL installation: https://docs.rs/openssl/latest/openssl/#automatic
2. Cargo, which can be installed using [rustup](https://rustup.rs/)
3. Clippy, which is a component that should be added through rustup.

orogene is built against relatively recent `stable` versions of Rust. For a
specific version to install, refer to `Cargo.toml`'s `[package]` section, as
`rust-version`.

If you plan on builing wasm packages for orogene sub-crates, you'll also need:

1. `cargo install -f wasm-bindgen-cli`
2. `cargo install -f wasm-pack` (or installed [prebuilt binaries](https://rustwasm.github.io/wasm-pack/installer/))

### Building the CLI

Linux has a couple of build requirements. These can be installed on ubuntu
with the following command. Adapt as needed for your distro:

```sh
sudo apt-get install build-essential pkg-config libssl-dev
```

On Windows, you'll need a working `msvc`.

You can build the CLI using a plain `cargo build` (optionally with
`--release`), and binaries will be available in your local
`./target/{debug,release}/oro[.exe]` directory.

The `oro`/`oro.exe` is a standalone binary but, **on Linux**, does require a
valid installation of `openssl 1` installed on the system in order to run.

### Building WASM packages

There's currently two packages with support for WASM, only one of which should
really be used right now: `crates/nassun` and `crates/node-maintainer`. Both
of them are published to the NPM registry as part of `orogene`'s release
process.

`node-maintainer` is orogene's dependency resolver and is responsible for
calculating dependency trees based on package.json, lockfiles, etc. Extraction
to filesystems is not supported in WASM mode because it is built for
`wasm32-unknown-unknown`, not `wasm-wasi`. `node-maintainer` also exports
`nassun`'s APIs under the same wasm package, so it's not recommended to
install _both_ `nassun` and `node-maintainer` wasm packages.

To build `node-maintainer`:

1. Make sure you're in the orogene project root.
2. `wasm-pack build crates/node-maintainer --target web`
3. There will now be an NPM package under `crates/node-maintainer/pkg` that
   can be packed, copied (to vendor it somewhere, etc).

## Working on the installer

Orogene's "install" command is called `apply`. During development, you can
call it like this:

```sh
$ cargo run [--release] -- apply [--root ../path/to/test/react-app] [--oro-cache ./path/to/cache] [--loglevel info]
```

It might be worth it to run with `--release` every time if you plan on seeing
applies to completion, because debug builds of orogene are pretty slow.

The apply command itself is defined in `src/commands/apply.rs`, and is
pretty much a UI wrapper around `crates/node-maintainer`'s APIs.

If you're only interested in the dependency resolution aspect of `apply`,
you can run with `--lockfile-only`, which will skip the much more expensive
extraction process.

## Logging

orogene uses [tracing](https://docs.rs/tracing) and
[tracing-subscriber](https://docs.rs/tracing-subscriber) for tracing and
logging capabilities, which can be configured through `--loglevel`. The syntax
for this option is documented
[here](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives).
For example, `cargo run -- apply --loglevel node_maintainer=trace,info` will
default to INFO-level logging for everything, but also include TRACE-level
logging for anything logged by `node-maintainer` (node the underscore. It's
based on the module name, not the crate.)

## Tagging and Releasing New Versions

Releasing orogene is a four-step process:

1. First, we make sure all the READMEs are up-to-date.
2. Then, a changelog is generated, editorialized, and commited.
3. After that, a command is run to release the entire repository in lockstep.
4. Finally, an automated process takes care of generating a GitHub release and
building prebuilt binaries for distribution.

In order to follow these steps, you'll need to intall `cargo-make`:

```sh
$ cargo install -f cargo-make
```

### Generate/Update READMEs

First, make sure `crates/README.md` is up-to-date with the latest crates
available under `crates/`, in case any of them have been added, removed, or
had significant description changes.

```sh
$ cargo make readmes
```

This command will update and/or update crate READMEs based on rustdoc from
their main modules.

If there were any changes, add and commit them with `git add` and `git commit
-m 'docs: update READMEs'`

### Changelog

```sh
$ cargo make changelog X.Y.Z
```

Where `X.Y.Z` is the full target semver version of the intended upcoming
release.

This will automatically update `CHANGELOG.md` with commits based on
conventional-commits in the git log.

If any additional messages are to be included, please add them under the new
version header.

`git add CHANGELOG.md` and `git commit -m 'docs: update changelog'` to
complete this step.

### Tag and Release

First, do a quick dry run:

```sh
$ cargo make release X.Y.Z
```

If everything looks good, execute it for real. You'll need to make sure you're
logged into both GitHub and crates.io and allowed to publish and push to
GitHub main:

```sh
$ cargo make release X.Y.Z --execute
```

This will take care of publishing all the crates in the repository under
version `X.Y.Z` to crates.io, tagging a new git tag, and pushing everything to
GitHub when it's done.

### GitHub Release

orogene uses [cargo-dist](https://opensource.axo.dev/cargo-dist/) for
generating prebuilt binaries and GitHub Releases. No intervention is necessary
here. This is handled by a GitHub Action configured under
[`.github/workflows/release.yml`](https://github.com/orogene/orogene/actions/workflows/release.yml).

It may take some time for everything to build, but the final release will be
made available under `https://github.com/orogene/orogene/releases/tag/vX.Y.Z`,
at which point the new version may be announced.

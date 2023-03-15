<div class="oranda-hide">

# orogene

</div>

> Yet another `node_modules/` package manager, I guess.

[![crates.io](https://img.shields.io/crates/v/orogene.svg)](https://crates.io/crates/orogene)
[![GitHub checks
state](https://img.shields.io/github/checks-status/orogene/orogene/main)](https://github.com/orogene/orogene/actions/workflows/ci.yml?query=branch%3Amain)
[![Project
Roadmap](https://img.shields.io/badge/Roadmap-Project%20Roadmap-informational)](https://github.com/orgs/orogene/projects/2/views/1)

Orogene is a next-generation package manager for tools that use
`node_modules/`, such as bundlers, CLI tools, and Node.js-based
applications. It's fast, robust, and meant to be easily integrated into
your workflows such that you never have to worry about whether your
`node_modules/` is up to date.

> *Note*: Orogene is still under heavy development and shouldn't be
> considered much more than a tech demo or proof of concept. Do not use in
> production yet.

### Benchmarks

Even at this early stage, orogene is **very** fast. These benchmarks are all
on ubuntu linux running under wsl2, with an ext4 filesystem.

All benchmarks are ordered from fastest to slowest (lower is better):

#### Warm Cache

This test shows performance when running off a warm cache, with an existing
lockfile. This scenario is common in CI scenarios with caching enabled, as
well as local scenarios where `node_modules` is wiped out in order to "start
over" (and potentially when switching branches).

Of note here is the contrast between the subsecond (!) installation by
orogene, versus the much more noticeable install times of literally everything
else.

| Package Manager | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `orogene` | 417.3 ± 43.3 | 374.6 | 524.8 | 1.00 |
| `bun` | 1535.2 ± 72.5 | 1442.3 | 1628.9 | 3.68 ± 0.42 |
| `pnpm` | 8285.1 ± 529.0 | 7680.4 | 9169.9 | 19.85 ± 2.42 |
| `yarn` | 20616.7 ± 1726.5 | 18928.6 | 24401.5 | 49.41 ± 6.59 |
| `npm` | 29132.0 ± 4569.2 | 25113.4 | 38634.2 | 69.81 ± 13.13 |

#### Cold Cache

This test shows performance when running off a cold cache, but with an
existing lockfile. This scenario is common in CI scenarios that don't cache
the package manager caches between runs, and for initial installs by
teammates on relatively "clean" machines.

| Package Manager | Mean [s] | Min [s] | Max [s] | Relative |
|:---|---:|---:|---:|---:|
| `bun` | 5.203 ± 1.926 | 3.555 | 9.616 | 1.00 |
| `orogene` | 8.346 ± 0.416 | 7.938 | 9.135 | 1.60 ± 0.60 |
| `pnpm` | 27.653 ± 0.467 | 26.915 | 28.294 | 5.31 ± 1.97 |
| `npm` | 31.613 ± 0.464 | 30.930 | 32.192 | 6.08 ± 2.25 |
| `yarn` | 72.815 ± 1.285 | 71.275 | 74.932 | 13.99 ± 5.19 |

#### Caveat Emptor

At the speeds at which orogene operates, these benchmarks can
vary widely because they depend on the underlying filesystem's performance.
For example, the gaps might be much smaller on Windows or (sometimes) macOS.
They may even vary between different filesystems on Linux/FreeBSD. Note that
orogene uses different installation strategies based on support for e.g.
reflinking (btrfs, APFS, xfs).

### Building

#### Requirements

You will need a Rust toolchain installed. See [the official Rust docs for
instructions](https://www.rust-lang.org/tools/install). And
[git](https://git-scm.com/downloads). Next, get a checkout of the source:

```
git clone https://github.com/orogene/orogene.git
cd orogene
```

#### Building

Your first build:

```
cargo build
```

The first time you run this, this downloads all the dependencies you will
need to build orogene automatically. This step might take a minute or two,
but it will only be run once.

Then it compiles all the dependencies as well as the orogene source files.

It should end with something like:

```
…
Finished dev [unoptimized + debuginfo] target(s) in 1m 22s
```

When you’ve made changes to the orogene source code, run `cargo build`
again, and it will only compile the changed files quickly:

```
cargo build
   Compiling orogene v0.1.0 (/Users/jan/Work/rust/orogene)
    Finished dev [unoptimized + debuginfo] target(s) in 2.41s
```

#### Running

After building successfully, you can run your build with `cargo run`. In
the default configuration, this will run an `oro` executable built for
your local system in `./target/debug`. When you run it, it shows you a
helpful page of instructions of what you can do with it. Give it a try:

```
    Finished dev [unoptimized + debuginfo] target(s) in 0.14s
     Running `target/debug/oro`
`node_modules/` package manager and utility toolkit.

Usage: oro [OPTIONS] <COMMAND>

Commands:
  ping     Ping the registry
  resolve  Resolve a package tree and save the lockfile to the project directory
  restore  Resolves and extracts a node_modules/ tree
  view     Get information about a package
  help     Print this message or the help of the given subcommand(s)

Options:
      --root <ROOT>          Package path to operate on
      --registry <REGISTRY>  Registry used for unscoped packages
      --cache <CACHE>        Location of disk cache
      --config <CONFIG>      File to read configuration values from
      --loglevel <LOGLEVEL>  Log output level/directive
  -q, --quiet                Disable all output
      --json                 Format output as JSON
  -h, --help                 Print help (see more with '--help')
  -V, --version              Print version
```

That’s it for now, happy hacking!

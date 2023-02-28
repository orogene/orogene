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

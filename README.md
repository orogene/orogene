# orogene ![CI](https://github.com/orogene/orogene/workflows/CI/badge.svg)

Yet another JavaScript package manager, I guess.

## Building

### Requirements

You will need a Rust toolchain installed. See [the official Rust docs for
instructions](https://www.rust-lang.org/tools/install). And
[git](https://git-scm.com/downloads). 
Next, get a checkout of the source:

```shell
git clone https://github.com/orogene/orogene.git
cd orogene
```

### Building

Your first build:

```
cargo build
```

The first time you run this, this downloads all the dependencies you will need
to build orogene automatically. This step might take a minute or two, but it
will only be run once.

Then it compiles all the dependencies as well as the orogene source files.

It should end with something like:

```
…
Finished dev [unoptimized + debuginfo] target(s) in 1m 22s
```

When you’ve made changes to the orogene source code, run `cargo build` again,
and it will only compile the changed files quickly:

```shell
cargo build
   Compiling orogene v0.1.0 (/Users/jan/Work/rust/orogene)
	Finished dev [unoptimized + debuginfo] target(s) in 2.41s
```

### Running

After building successfully, you can run your build with `cargo run`. In the
default configuration, this will run an `oro` executable built for your local
system in `./target/debug`. When you run it, it shows you a helpful page of
instructions of what you can do with it. Give it a try:

```shell
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

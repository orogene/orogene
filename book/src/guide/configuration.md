# Configuration

## Specifying Configurations

Orogene configuration happens in three different ways, in order of precedence:

1. Direct command line flags (`--foo blah`), which can be negated (`--no-foo`)
2. [KDL](https://kdl.dev) [configuration file(s)](#configuration-files) (`oro.kdl`)
3. Environment variables, prefixed by `oro_config_` (`oro_config_foo=blah`)

## Configuration Files

There are three possible configuration file locations:

1. `oro.kdl`, located in the root of the current project.
2. `oro.kdl`, located in a [system-dependent
   location](https://docs.rs/directories/latest/directories/struct.ProjectDirs.html#method.config_dir)
3. Any file specified by using the global `--config FILE` CLI option. When
   this option is specified, neither of the other configuration files are
   loaded, and file-based configuration happens entirely off the given file.
   Environment variables and command line options will still be loaded
   normally.

All files files use [KDL](https://kdl.dev) as their configuration language.
Key/value options are specified using `node-name "value"` format. For example,
`foo "bar"` would be the equivalent of `--foo bar`. For boolean operations,
use `foo true` and `foo false`. Negations (`no-foo`) are not supported.

## Available Options

All available options for individual commands are available by doing `oro
<subcommand> --help`, or by visiting the individual commands' documentation
pages on this site. "Global Options" can potentially be used across all
commands and have a shared meaning.

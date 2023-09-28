# Configuration

## The `oro.kdl` Config File

Options are read from three possible `oro.kdl` file locations:

1. `oro.kdl`, located in the root of the current project.
2. `oro.kdl`, located in a [system-dependent
   location](https://docs.rs/directories/latest/directories/struct.ProjectDirs.html#method.config_dir)
3. Any file specified by using the global `--config FILE` CLI option. When
   this option is specified, neither of the other configuration files are
   loaded, and file-based configuration happens entirely off the given file.
   Environment variables and command line options will still be loaded
   normally.

All files files use [KDL](https://kdl.dev) as their configuration language.
Key/value options are typically specified using `node-name "value"` format.
For example, `foo "bar"` would be the equivalent of `--foo bar`. For boolean
operations, use `foo true` and `foo false`. If an empty node name is found,
that will be treated as if it had a value of `true`. Negations (`no-foo`) are
not supported.

Some configurations, such a [Options](#options-from-orokdl), exist in nested
nodes. Refer to their dedicated sections for more details.

## Options

In Orogene, "options" refers to configurations that can be provided through
the command line.

### Available Options

All available options for individual commands are available by doing `oro
<subcommand> --help`, or by visiting the individual commands' documentation
pages on this site.

"Global Options" can potentially be used across all commands and have a shared
meaning. Global options are always listed and accepted for all commands, even
if the individual commands do not make use of them.

Commands with an "Apply Options" section support [implicit dependency
application](./node_modules.md). Any that don't have that section do not
interact with `node_modules` at all.

### Specifying Options

Orogene options can be provided in three different ways, in order of precedence:

1. Direct command line flags (`--foo blah`), which can be negated (`--no-foo`)
2. [KDL](https://kdl.dev) [configuration file(s)](#configuration-files)
   (`oro.kdl`), inside the `options` node.
3. Environment variables, prefixed by `oro_config_` (`oro_config_foo=blah`)

## Options from `oro.kdl`

Options can be specified through `oro.kdl` by using the toplevel `options`
node. For example:

```kdl
// ./oro.kdl
options {
    registry "https://my.private.registry/_path"
    emoji false
}
```

# Crates

This is where you'll find the internal libraries that together form `oro`.

* [cacache](./cacache) content-addressable cache behind oro, where downloaded packages are stored. Currently a vendored and tweaked version of [cacache-rs](https://github.com/zkat/cacache-rs).
* [oro-classic-resolver](./oro-classic-resolver)  Resolves PackageRequests to Packages using the versions declared in the packument
* [oro-client](./oro-client) an HTTP client based on surf with connection pooling
* [oro-command-derive](./oro-command-derive) a proc-macro that provivdes an implementation for configuring oro commands
* [oro-command](./oro-command) contains the two traits for commands: their execution (`OroCommand`) and their configuration (`OroCommandLayerConfig`)
* [oro-config](./oro-config) contains functionality about a global configuration
* [oro-tree](./oro-tree) functionality to read and parse `package-lock.json` files
* [package-arg](./package-arg) parsing and validating package arguments such as `npm:@hello/world@1.2.3`. Used by `rogga` and `restore` at the moment
* [rogga](./rogga) resolves and fetches packages either from a registry (using `oro-client` and NPM) or locally

# Commands

This is where you'll find the implementation of the `oro` commands.
The commands are meant to feel familiar to users of `npm`, `yarn`, the entropic client `ds`, `tink`, and others.

There are three parts to commands:

 * The datastructure with `clap` annotations that is used for command line arguments.
 * The implementation of `OroCommand` that contains the actual exectuion of the commands.
 * The implementation of `OroCommandLayerConfig` that is used to pass configuration details to the command.

The last point can be skipped if you use `#[derive(OroCommand)]` on the struct.
If there are fields that should be skipped from auto-configuration, you can ignore them with `#[oro_config(ignore)]`.

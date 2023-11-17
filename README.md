# Pacwrap

![pacwrap](./docs/pacwrap.png "pacwrap")

Pacwrap provides a package management front-end with libalpm to facilitate the creation of container
filesystems with filesystem deduplication. Sandboxing is also provided via bubblewrap to run applications 
inside of these containers. CLI and GUI applications are all supported.

## Example usage

To create a container, execute the following command:

```
$ pacwrap -Syucb --target=base
```

Then to launch a shell inside of this container to configure it:

```
$ pacwrap -Es base
```

And then finally, to install neovim inside of a fresh, replicable, root container:


```
$ pacwrap -Syucr --target=neovim neovim --target=base
```

More advanced examples along with further documentation of configuration can be found further 
elaborated upon **[here](./docs/README.md)**.

## Manual

An online version of the user manual is viewable **[here](./docs/manual.md)**.

## Build requirements

A minimum version of Rust 1.70, with base-devel and repose packages from Arch Linux's repositories.

## Distribution support

Currently only Arch Linux is supported in containers as package management is faciliated by libalpm.
However, this package should be distribution agnostic, so it should be possible to use on non-Arch-based distributions.

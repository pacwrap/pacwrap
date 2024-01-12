# pacwrap

<img align="left" src="./assets/logo.svg">

A package management front-end which utilises libalpm to facilitate the creation of unprivileged, userspace containers with parallelised, filesystem-agnostic deduplication. Sandboxing of unprivileged namespace containers is provided via bubblewrap to execute package transactions and launch applications inside of these containers.

This application is designed to allow for the creation and execution of secure, replicable containerised environments for general-purpose use. CLI and GUI applications are all supported*. Once a container environment is configured, it can be re-established or replicated on any system. 

Goal of this project is to provide a distribution-backed alternative to flatpak with easily configurable security parameters.

\* Some CLI-based applications, such as ncspot, require disabling termios isolation. This could allow an attacker to overtake the terminal and thus breakout of the container.
## Example usage

To create a base container, execute the following command:

```
$ pacwrap -Syucb --target=base
```

Then to launch a shell inside of this container to configure it:

```
$ pacwrap -Es base
```

And finally, to install neovim inside of a fresh, aggregated container:


```
$ pacwrap -Syucat neovim --dep=base
```

More advanced examples along with further documentation of configuration can be found further 
elaborated upon **[here](./docs/README.md)**.

## Manual

An online version of the user manual is viewable **[here](./docs/manual.md)**.

## Build requirements

A minimum version of Rust 1.72, with base-devel and repose packages from Arch Linux's repositories.

## Distribution support

Currently only Arch-based distributions ares supported as package management is faciliated by libalpm. However, this package aims to be distribution agnostic, so it should be possible in future to use on non-Arch-based distributions.

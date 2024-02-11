# pacwrap

<img align="left" src="./assets/logo.svg">

A package management front-end which utilises libalpm to facilitate the creation of unprivileged, userspace containers with parallelised, filesystem-agnostic deduplication. These containers are constructed via bubblewrap to execute package transactions and launch applications.

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

And finally, to install neovim inside of a fresh, aggregated container called ```editor```:


```
$ pacwrap -Syucat editor --dep=base neovim
```

More advanced examples along with further documentation of configuration can be found further 
elaborated upon **[here](./docs/)**.

## Features

Since this project is a work in progress, net everything is yet completed. Please refer to the matrix below for further detail. 

If a feature you see here is not completed, feel free to submit a PR; or submit an issue regarding a feature not listed herein for triage.

| Feature                            | Description                                                                 | Status        |
| :---                               | :---                                                                        |     :----:    |
| Aggregate Transactions             | Aggregate package transactions across containers                            | ✅            |
| Transaction Agent                  | Transact within a sandboxed runtime environment                             | ✅            |
| Transaction CLI                    | Functional                                                                  | ✅            |
| Global Configuration               | Functional                                                                  | ✅            |
| Dependency Resolution              | Functional, but too liberal to compensate for a lack of conflict resolution | ⚠            |
| Foreign Database Resolution        | Populates foreign package database in aggregate containers                  | ✅            |
| Foreign Database Resolution (Lazy) | Not yet implemented                                                         | ❌            |
| Conflict Resolution                | Not yet implemented                                                         | ❌            |
| Package Installation               | Functional                                                                  | ✅            |
| Package Removal                    | Functional                                                                  | ✅            |
| Desktop Entry Creation             | ```pacwrap-utils``` at present provides this via ```pacwrap -Ud```          | ⚠            |
| Container Execution                | Functional                                                                  | ✅            |
| Launch within existing namespace   | Not yet implemented                                                         | ❌            |
| Container Configuration            | Functional                                                                  | ✅            |
| Container Creation                 | Functional                                                                  | ✅            |
| Container Runtime                  | Embedded runtime environment                                                | ✅            |
| Container Schema                   | Container filesystem schema with version tracking                           | ✅            |
| Filesystem Deduplication           | Retains filesystem state across containers with hardlinks                   | ✅            |
| Seccomp Filters                    | Application of seccomp filters to instances via libseccomp bindings         | ✅            |
| Dbus Isolation                     | Functional - provided by xdg-dbus-proxy                                     | ✅            |
| Networking Isolation               | Not yet implemented                                                         | ❌            |
| Port to Rust                       | Script: pacwrap-utils                                                       | ⚠            |
| Configuration CLI (user friendly)  | Not yet implemented                                                         | ❌            |
| Process API                        | Container process enumeration                                               | ✅            |
| Process CLI                        | Functional                                                                  | ✅            |
| Utility CLI (native)               | Not yet implemented                                                         | ❌            |
| Localization                       | Not yet implemented                                                         | ❌            |

## Manual

An online version of the user manual is viewable **[here](./docs/manual.md)**.

## Build requirements

A minimum version of Rust 1.72 is required to build with the following libraries fulfilled by your distribution:
```
libalpm, libseccomp
```

## Packaging requirements

The following Arch Linux packages (or your distribution's equivalent) are required for build-time artefacts: 
```
bash, busybox, coreutils, fakeroot, fakechroot
```

## Distribution support

Although this project aims to be distribution agnostic, at present only Arch-based distributions are supported. This project does aim, however, to be distribution agnostic, so in future it should be possible to support other distributions.

# Getting started

Be sure to read the manuals **[here](./config.md)** and **[here](./manual.md)**.

## Creating containers

To create a base container, execute the following command:

```
$ pacwrap -Syucbt base
```

Then to create a container segment named ```common``` with a common set of packages, and an aggregate container named ```steam``` 
built up of these two containers, execute the following command sequence:

```
$ pacwrap -Syucst common mesa gtk3 nvidia-utils -cat steam steam --dep=base,common
```

And finally, to launch ```steam``` inside of a fresh, aggregated container:

```
$ pacwrap run steam steam
```

You might've noticed that last step didn't work. That's because each container is locked down with a tight permission set by default.

## Container configuration

Then you might be wondering: How do I use this with anything?

To explain that, first a little bit of background: Pacwrap implements a DSL (Domain Specific Language) with YAML. 
Containers are configured with this DSL to permiss access to filesystems, devices, UNIX sockets, networking, etc..

For example, here's a sample configuration of a container environment used for playing Steam games:

```
container_type: Aggregate
dependencies:
- base
- common
explicit_packages:
- steam
meta_version: 1706515223
enable_userns: true
retain_session: false
seccomp: true
allow_forking: true
filesystems:
- mount: root
- mount: home
- mount: sysfs
- mount: to_root
  volumes:
  - path: /usr/share/icons
  - path: /usr/share/fonts
  - path: /etc/fonts
  - permission: rw
    path: /media/Storage/Games/Steam
    dest: /mnt/SteamLibrary
- mount: to_home
  volumes:
  - path: .config/fontconfig/
permissions:
- module: net
- module: gpu
- module: display
- module: pulseaudio
- module: dev
  devices:
  - input
- module: env
  variables:
  - var: LANG
  - var: QT_X11_NO_MITSHM
    set: '1'
dbus:
- module: appindicator
```

## Configuration Modules

Each ```base``` and ```aggregate``` type container can make use of filesystems, permissions, and dbus modules. 
These provide a good, ergonomic way to abstract these problems, whilst minimising complexity, providing a flexible human-readable configuration language.

You then might wonder: What are the individual modules, and what do each of them do? Here's a small breakdown of what each them do below:

### Networking module

```
- module: net
```

This module instructs bubblewrap to provide host networking to the container.

### Display module

```
- module: display
```

The display module detects, validates, and provides an X11 or Wayland (if available) display socket to the container.

### GPU module

```
- module: gpu
```

And then this module, binds your system's graphics devices to the container.

### Home bind module

```
- mount: to_home
  volumes:
  - permission: rw
    path: Documents
  - permission: ro
    path: .config/fontconfig
```

We then mount our Documents folder with read/write permissions and our .config/fontconfig directory with read-only permissions.

## Locations

- Configuration files: ```~/.config/pacwrap/```
- Container data stores: ```~/.local/share/pacwrap/```
- Package caches: ```~/.cache/pacwrap/pkg/```

## Further documentation

Documentation on each module, breaking down the individual options, can be found **[here](./modules/)**.

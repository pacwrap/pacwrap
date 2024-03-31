# Mount home

Mount the container's home directory into the container.

## Example

```
filesystems:
- mount: home
```

## Description

Provides the binding for the container's home directory. By default, this module
will mount `~/.local/share/pacwrap/home/[container_name]` to `$HOME` inside the
container at `/home/[container_name]`. Please refer to the [**to_home**](./to_home.md) 
module for more advanced options.

This module plays an important role in initializing the container's runtime environment.

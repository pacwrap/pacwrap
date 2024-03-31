# Mount root

Mount the container's home directory into the container.

## Example

```
filesystems:
- mount: root
```

## Description

Provides the binding for the container's root filesystem. By default, this module
will bind the minimum required at `~/.local/share/pacwrap/root/[container_name]`
to allow for a functional userspace inside of the container. Please refer to the
[**to_root**](./to_root.md) module for more advanced options.

This module plays an important role in initializing the container's runtime environment.

# Mount root

Mount the container's home directory into the container.

## Example

Mount root filesystem with the addition of the `/opt` directory.

```
filesystems:
- mount: root
  volumes:
    path: /opt
```

## Description

Provides a binding for the container's root filesystem. By default, this module will bind the minimum required 
at `~/.local/share/pacwrap/root/[container_name]`to allow for a functional userspace inside of the container. 

Mount additional directories to the container's root by specifying a `path` key with a directory as the key; the
`permission` and `dest` keys are ignored. Pacwrap will terminate with an error condition if the file or directory is 
not found, or if the user is otherwise insufficiently privileged.

Refer to the [**to_root**](./to_root.md) module for more advanced options.

Please note: This module plays an important role in initializing the container's runtime environment.

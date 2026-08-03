# Mount to XDG Runtime Directory

Mount filesystem volumes to the container's XDG Runtime directory.

## Example

Mount `$XDG_RUNTIME_DIR/keyring` to the container's `$XDG_RUNTIME_DIR` directory as read-only:

```
filesystems:
- mount: to_xdg_runtime
  volumes:
  - path: keyring
```

## Description

Mount filesystem volumes from the user's `$XDG_RUNTIME_DIR` into the container, in the container's 
`$XDG_RUNTIME_DIR` directory, unless the destination is otherwise specified with `dest` key. Pacwrap 
will terminate with an error condition if the file or directory is not found, or the user is otherwise 
insufficiently privileged.

Specify read/write permissions with a string using the `permission` key. Valid combinations are 
abbreviated as following: `ro` for read-only, and `rw` for read-write permissions.

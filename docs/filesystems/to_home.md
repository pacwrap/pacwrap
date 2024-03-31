# Mount to home

Mount filesystem volumes to the container's root directory.

## Example

Mount `~/.config/fontconfig/ to the container's `$HOME` directory as read-only; then mount the 
`~/Downloads` to the container's `$HOME` directory with read-write permissions.

```
filesystems:
- mount: to_home
  volumes:
  - path: .config/fontconfig
  - permission: rw
    path: Downloads
```

## Description

Mount filesystem volumes from the runtime user's `$HOME` into the container, in the container's `$HOME`
directory, unless the destination is otherwise specified with `dest` variable. Pacwrap will terminate 
with an error condition if the file or directory is not found; or the user is otherwise not sufficiently 
privileged.

Specify read/write permissions with a string using the `permission` variable. Valid combinations are 
abbreviated as following: `ro` for read-only, and `rw` for read-write permissions.

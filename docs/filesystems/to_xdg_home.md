# Mount to XDG Home Directory.

Mount XDG directories from the user's home directory into the container, utilising the user's `$HOME` path.

## Example

Mount `$HOME/Games` directory directly to the container with read-write permissions:

```
filesystems:
- mount: to_xdg_home
  volumes:
  - permission: rw
    path: Games 
```

## Description

By default, this module will mount the following directories from the `$HOME` path: `Downloads`, `Documents`, 
`Pictures`, `Videos`, and `Music`. This provides seamless file chooser integration support for XDG Portals.

Mount filesystem volumes from the user's `$HOME` path directly into the container, unless the destination 
is otherwise specified with `dest` key; the value is relative to the path root. Pacwrap will terminate with 
an error condition if the file or directory is not found, or the user is otherwise insufficiently privileged.

Specify read/write permissions with a string using the `permission` key. Valid combinations are 
abbreviated as following: `ro` for read-only, and `rw` for read-write permissions.

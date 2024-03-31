# Mount to root

Mount filesystem volumes to the container's root directory.

## Example

Mount `/usr/share/icons/`, and `/etc/fonts/`, as read-only to the container; then mount
`/media/Storage/Games/Steam` to `/mnt/SteamLibrary`/ in the container with read-write permissions.

```
filesystems:
- mount: to_root
  volumes:
  - path: /usr/share/fonts
  - path: /etc/fonts
  - permission: rw
    path: /media/Storage/Games/Steam 
    dest: /mnt/SteamLibrary
```

## Description

Mount filesystem volumes from the host into the container, at the destination mirroring the host, unless 
the destination is otherwise specified with `dest` variable. Pacwrap will terminate with an error condition 
if the file or directory is not found; or the user is otherwise not sufficiently privileged.

Specify read/write permissions with a string using the `permission` variable. Valid value combinations are 
abbreviated as following: `ro` for read-only, and `rw` for read-write permissions.

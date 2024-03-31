# Device Module

Permiss access to devices from the `/dev/` sysfs.

## Example

Permiss access to the /dev/input device in the container environment.

```
- module: dev
  devices:
  - input
```

## Description

Mount a list of devices as specified in the `devices` array. Requires the user
have unprivileged access to the device in question. Otherwise this module will
terminate pacwrap with an error condition.

In most cases, the automatic permissions should be sufficient with the exception 
of some USB input devices for particular use cases.

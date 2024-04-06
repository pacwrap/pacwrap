# Socket Module

Avail namespace to xdg-dbus-proxy instance.

## Example

Grant TALK access to org.free.desktop.secrets namespace.

```
dbus:
- module: socket
  policy: TALK
  address:
  - org.freedesktop.secrets
```

## Description

Socket module allows permission to be granted through the `xdg-dbus-proxy` job managed by pacwrap
to grant selective access to dbus namespaces on the host dbus session bus.

See the [**contents**]!(./README.md) for modules with pre-defined dbus policies.

# Environment Variables Module

Set environment variables in the container environment.

## Example

Passthrough the environment variable `LANG`, and set the environment variable `QT_X11_NO_MITSHM`.

```
permissions:
- module: env
  variables:
  - var: LANG
  - var: QT_X11_NO_MITSHM
    set: '1'
```

## Description

Use this module to passthrough or define environment variables in the container environment.
When an environment variable is not available in the host environment, this module will
provide warning and set a blank value in the target container.

By default pacwrap, will not passthrough environment variables from the host, unless specified
otherwise by the user with the elidation of the `set` parameter when definining a `var`.

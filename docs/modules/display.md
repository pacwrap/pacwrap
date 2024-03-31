# Display Module

Provide access to display server sockets.

## Example
```
permissions:
- module: display
```

## Description

Use this module to automatically detect and bind available display server sockets to the container environment.
If no sockets are available, this module will terminate pacwrap with an error condition.

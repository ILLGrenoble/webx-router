# WebX Router

## Description

The WebX Router manages multiple WebX sessions, routing requests, instructions and messages between running WebX Engines and the WebX Relay.

In collaboration with the WebX Session Manager, X11 sessions are initiated for new clients and WebX Engines are launched with corresponding display and xauth environments.

The WebX Router manages multiple ZeroMQ sockets to communicate with the WebX Relay, the WebX Session Manager and the multiple WebX Engines. ZeroMQ is also used for internal messaging.

## Compiling


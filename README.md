# WebX Router

## Description

The WebX Router manages WebX sessions in a multiuser environment, routing requests, instructions and messages between running WebX Engines and the WebX Relay.

It uses the WebX Session Manager to authenticate user connection requests and to spawn Xorg and window manager processes. WebX Engines are then launched by the Router with corresponding DISPLAY and XAUTHORITY environment variables.

The WebX Router manages multiple ZeroMQ sockets to communicate with the WebX Relay (TCP), the WebX Session Manager (IPC) and the multiple WebX Engines (IPC). ZeroMQ is also used for internal messaging (Inproc).

### Included in this project

This project includes:
 - The WebX Router Rust source code
 - VSCode Launch commands
 - Dockerfiles to build the WebX Router and package it in a Debian Package
 - Github actions to buid Debian Packages and add them to releases

## WebX Overview

WebX is a Remote Desktop technology allowing an X11 desktop to be rendered in a user's browser. It's aim is to allow a secure connection between a user's browser and a remote linux machine such that the user's desktop can be displayed and interacted with, ideally producing the effect that the remote machine is behaving as a local PC.

WebX's principal differentiation to other Remote Desktop technologies is that it manages individual windows within the display rather than treating the desktop as a single image. A couple of advantages with a window-based protocol is that window movement events are efficiently passed to clients (rather than graphically updating regions of the desktop) and similarly it avoids <em>tearing</em> render effects during the movement. WebX aims to optimise the flow of data from the window region capture, the transfer of data and client rendering.

> The full source code is openly available and the technology stack can be (relatively) easily demoed but it should be currently considered a work in progress.

The WebX remote desktop stack is composed of a number of different projects:
 - [WebX Engine](https://github.com/ILLGrenoble/webx-engine) The WebX Engine is the core of WebX providing a server that connects to an X11 display obtaining window parameters and images. It listens to X11 events and forwards event data to connected clients. Remote clients similarly interact with the desktop and the actions they send to the WebX Engine are forwarded to X11.
 - [WebX Router](https://github.com/ILLGrenoble/webx-router) The WebX Router manages multiple WebX sessions on single host, routing traffic between running WebX Engines and the WebX Relay. 
 - [WebX Session Manager](https://github.com/ILLGrenoble/webx-session-manager) The WebX Session manager is used by the WebX Router to authenticate and initiate new WebX sessions. X11 displays and desktop managers are spawned when new clients successfully authenticate.
 - [WebX Relay](https://github.com/ILLGrenoble/webx-relay) The WebX Relay provides a Java library that can be integrated into the backend of a web application, providing bridge functionality between WebX host machines and client browsers. TCP sockets (using the ZMQ protocol) connect the relay to host machines and websockets connect the client browsers to the relay. The relay transports data between a specific client and corresponding WebX Router/Engine.
 - [WebX Client](https://github.com/ILLGrenoble/webx-client) The WebX Client is a javascript package (available via NPM) that provides rendering capabilities for the remote desktop and transfers user input events to the WebX Engine via the relay.

To showcase the WebX technology, a demo is available. The demo also allows for simplified testing of the WebX remote desktop stack. The projects used for the demo are:
 - [WebX Demo Server](https://github.com/ILLGrenoble/webx-demo-server) The WebX Demo Server is a simple Java backend integrating the WebX Relay. It can manage a multiuser environment using the full WebX stack, or simply connect to a single user, <em>standalone</em> WebX Engine.
 - [WebX Demo Client](https://github.com/ILLGrenoble/webx-demo-client) The WebX Demo Client provides a simple web frontend packaged with the WebX Client library. The demo includes some useful debug features that help with the development and testing of WebX.
 - [WebX Demo Deploy](https://github.com/ILLGrenoble/webx-demo-deploy) The WebX Demo Deploy project allows for a one line deployment of the demo application. The server and client are run in a docker compose stack along with an Nginx reverse proxy. This provides a very simple way of connecting to a running WebX Engine for testing purposes.

 The following projects assist in the development of WebX:
 - [WebX Dev Environment](https://github.com/ILLGrenoble/webx-dev-env) This provides a number of Docker environments that contain the necessary libraries and applications to build and run a WebX Engine in a container. Xorg and Xfce4 are both launched when the container is started. Mounting the WebX Engine source inside the container allows it to be built there too.
 - [WebX Dev Workspace](https://github.com/ILLGrenoble/webx-dev-workspace) The WebX Dev Workspace regroups the WebX Engine, WebX Router and WebX Session Manager as git submodules and provides a devcontainer environment with the necessary build and runtime tools to develop and debug all three projects in a single docker environment. Combined with the WebX Demo Deploy project it provides an ideal way of developing and testing the full WebX remote desktop stack.

## Development

The WebX Router is designed to be built and run in a Linux environment and runs in connection with a WebX Session Manager process. It spawns WebX Engine processes that connect to Xorg displays. Development can be made directly on a linux machine providing the relevant libraries are installed or (as recommendd) development can be performed within a devcontainer.

### Building and running from source on a linux machine

The following assumes a Debian or Ubuntu development environment.

Install the following dependencies:

```
apt install curl gcc libzmq3-dev libclang-dev libpam-dev clang
```

Next, install the Rust language:

```
curl https://sh.rustup.rs -sSf > /tmp/rustup-init.sh \
    && chmod +x /tmp/rustup-init.sh \
    && sh /tmp/rustup-init.sh -y \
    && rm -rf /tmp/rustup-init.sh
```

Opening a new termminal should provide Rust's `cargo` build command, otherwise it can be located at `~/.cargo/bin/cargo`.

To compile the WebX Router, run the command: 

```
cargo build
```

The WebX Router can either be launched by using the VSCode Launch Command <em>Debug</em> or by launch in a terminal the following command:

```
./target/debug/webx-router
```

#### WebX Router configuration

The configuration file `config.yml` is used to define the logging level, TCP ports, IPC paths, WebX Engine path. This file can be located in the working directory or `/etc/webx/webx-router-config.yml`. Alternatively each configuration value can be overridden by an environment variable, prefixed by WEBX_ROUTER. For example, the `engine: path:` configuration value can be overridden by the environment variable `WEBX_ROUTER_ENGINE_PATH`.

### Building, running and debugging using the WebX Dev Workspace

The [WebX Dev Workspace](https://github.com/ILLGrenoble/webx-dev-env) combines the development of The WebX Engine, WebX Router and WebX Session Manager in a single workspace and the development and testing of all of these can be combined in a single devcontainer environment.

This is the recommended way of building, running and debuggine the WebX stack as it provides the most flexible approach to development without installing any dependencies. The environment is configured to easily run the three projects together and contains VSCode Launch Commands to debug the application.

In the devcontainer you should start by building the WebX Engine then launch the WebX Router and WebX Session Manager using the VSCode Launch Commands. The WebX Router can be debugged using the standard VSCode debugger.

Please refer to the project's README for more information.

### Running the WebX Demo to test the WebX Remote Desktop Stack

In a terminal on the host computer, the simplest way to test the WebX Router and its connection to the other projects is by running the [WebX Demo Deploy](https://github.com/ILLGrenoble/webx-demo-deploy) project. This runs the WebX Demo in a docker compose stack.

To fully test the WebX Stack run the demo as follows:

```
./deploy.sh
```

In a browser open https://localhost

You need to set the host of the WebX Server: running in a local devcontainer, set this to `host.docker.internal`.

Using the WebX Dev Workspace, you can log in with any of the pre-defined users (mario, luigi, peach, toad, yoshi and bowser), the password is the same as the username.

This will send the request to the WebX Router: the WebX Session Manager will authenticate the user and run Xorg and Xfce4 for the user; WebX Router then launches the locally-built webx-engine.

## Design

There are three main functionalities for the WebX Router:
 - Delegating requests for X11 sessions (including authentication and X11 session creation) to the WebX Session Manager
 - Managing a collection of WebX Engines (connected to the X11 sessions) on the same host
 - Routing instructions and messages between a WebX Relay and WebX Engines (based on a session Id that prefixes all instructions and messages)

### Socket connections

The WebX Router accepts connections from other hosts using ZeroMQ TCP sockets. Connections are made from the WebX Relay - typically there should only be a single WebX Relay connecting to the router.

Connections to the WebX Session Manager and WebX Engines are on the same host so use ZeroMQ IPC sockets (standard unix sockets). 

Messaging within the application is provided through ZeroMQ InProc internal messaging.

Four TCP sockets are opened by the WebX Router. These, and their default ports are:
 - Connection Initiation: 5555
 - Instruction Routing: 5556
 - Message Routing: 5557
 - Session Creation: 5558

#### Connection Initiation

A simple Client Connector TCP socket is provided allowing a relay to set up the other socket connections. Running using the request-response pattern (`ZMQ_REP`) this socket provides:
 - port details for the other sockets
 - public key for authentication encryption

#### Instruction and Message Routing

Two TCP sockets are used to manage instructions from the WebX Relay to WebX Engines and messages from the WebX Engines to the WebX Relay. 

The Relay Instruction Proxy runs with the subscriber pattern (`ZMQ_SUB`) on a TCP socket where it receives instructions from a relay and publishes it on an IPC socket (`ZMQ_PUB`). All WebX Engines on the host subscribe to this WebX Router publisher, filtering instructions by their sessionId: unfiltered instructions are handled by the engine.  

The Engine Message Proxy runs with the publisher pattern (`ZMQ_PUB`) on a TCP socket where it publishes messages (from WebX Engines) to the WebX Relay. Messages from the engines are received on an IPC subscriber socket (`ZMQ_SUB`) on the same host. 

The WebX Relay has filters messages by the sessionId and forwards them to the correct clients. 

#### Session Creation requests

Session creation requests occur using the Session Proxy. This socket runs the request-response pattern (`ZMQ_REP`) and is an encrypted TCP socket that allows for username and password to be passed from the relay.

With the creation command a connection to the WebX Session Manager is made (using another `ZMQ_REP` IPC socket) and a new X11 session requested (unless one already exists for the user).

##### Authentication over encrypted sockets

To avoid sending username and password over the network (even if private) the socket messages are encrypted using the [CURVE protocol](http://wiki.zeromq.org/build:encryption). A private and public key are generated by the WebX Router at startup. The public key is communicated to the WebX Relay to enable the encrypted traffic.

On the WebX host, internal unix sockets are used which are protected by unix file permissions.

Between the WebX Relay and the client it is assumed that HTTPS connections are used to protect the authentication credentials.

##### Pinging sessions

This socket is also used for liveliness messages to ensure the router and engines are running correctly.

Each WebX Relay connection will attempt to ping the connected WebX Engine.

The liveliness messages that are prefixed with a sessionId are forwarded to the WebX Engine on a specific IPC socket for each engine using a request-response (`ZMQ_REP`) socket pattern.

### Session management

The WebX Router maintains a collection of X11 sessions and associated WebX Engine. X11 session creation is delegated to the WebX Session Manager. A WebX Engine is spawned for the X11 session if necessary.

The WebX Session Manager generates a unique Id for each session: the WebX Router uses an environment variable when spawning a WebX Engine to forward this information to it. The Id is used to prefix all messages from an Engine and all instructions to it.

When receiving session creation requests from a particular user, the WebX Router verifies that the X11 session Id is always valid: if the WebX Session Manager returns a new session Id then the WebX router will spawn a new WebX Engine. 

#### Requests to WebX Session Manager

On receiving session creation requests with login credentials, a new socket connection is made to the WebX Session Manager. The login details along with the requested session screen width and height are sent as request parameters.

The WebX Session Manager validates the login credentials using the PAM services. Having successfully authenticated the user, it will determine if an X11 session is already running or whether a new one is required.

In its response, the WebX Session Manager communicated the following details for the X11 session:
 - the unique session id
 - user details (username and UID)
 - the DISPLAY id 
 - the XAUTHORITY path

#### Spawning WebX Engine sessions

Using the information returned by the WebX Session Manager the WebX Router will determine if a WebX Engine is running or not. 

If a new WebX Engine is required it spawns a new process using the user's UID as process owner.

Environment variables are set in the process to enable the WebX Engine to use the correct DISPLAY and XAUTHORITY values to allow it to securely connect to the X11 session.

The environment variables also inform the WebX Engine on the ZeroMQ IPC paths (for message publication and instruction subscriptions) and unique log file path.  

The WebX Engine is started with the keyboard layout chosen by the client.

### Multiple connections to same X11 session

> Note this section will change in the future

Currently a single WebX Engine is associated to an X11 Session. Multiple login actions from a WebX Relay will result in just a single connection to the WebX Router for a given host and only a single WebX Engine being spawned. 

While valid in terms of protocol this can produce network problems if the end clients have widely different bandwidths and large amounts of image data are produced by the WebX Engine: what is perfectly fine for a large bandwidth client may easily saturate a low bandwidth one.

Work is in progress to modify this to allow different clients to request different Remote Desktop qualities.



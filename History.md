1.5.4 08/09/2025
================
 * Add run_as_root to xorg settings to allow Xorg to be run optionally as the root user.

1.5.3 05/09/2025
================
 * Send resolution as env vars to window manager
 * run Xorg as root rather than with the user privileges (allows Xorg to run using nvidia drivers rather than xrdp driver)

1.5.2 30/07/2025
================
 * Fix bug on create_session_startup_thread stopping immediately.

1.5.1 08/07/2025
================
 * Modify the service so that it starts after the multi-user target rather than network.

1.5.0 07/07/2025
================
 * cli does async creation and then polls the status.
 * Add session_proxy action to get status of a session. Polling from clients allows them to determine when a session is running.
 * Add a timeout to sync session creation.
 * Handle create_async action in session_proxy. Check for ready Xorg in synchronous creation.
 * Check in thread for Xorg being ready to accept connections. Send status info of creation process to clients.
 * Async creation process of Xorg, window manager and engine (keeping legacy synchronous action): separate creation of xorg and wm. 
 * copy EnvList vars into a Vec so that it can be cloned easily. Store the AuthenticatedSession with the X11Session.
 * Refactoring management of x11_sessions into the x11_session_manager rather than xorg_service. Better mutex handling of session vector.
 * Refactoring: engine_session does not reference x11_session.
 * Refactoring to perform the authentication immediately from the session_proxy. The x11_session_manager only produces X11 processes (no auth).

1.4.2 02/07/2025
================
 * If requests to the engine fail then recreate the socket to it (sockets may be created prematurely).

1.4.1 01/07/2025
================
 * better handling of signals (sigterm, sigquit and sigint) to shutdown gracefully and kill spawned processes.
 * Remove session activity management: no auto deletion of sessions. This improves the efficiency of forwarding instructions. Session deletion can be handled by the relay.

1.4.0 30/06/2025
================
 * Use a session "secret" rather than session_id in communication to the webx engine so that it is invisible to all other users on the host. Only authenticated users of the session can know the secret.
 * update readme to include legacy session manager functionality.
 * Build a client (cli) application that can create and list webx sessions. Cli can repeatedly ping the engine to verify that it is running.
 * Update authentication to allow local file (owner and read-only by the user of the cli) with auto-generated password.
 * General improvements to error handling and addition of AuthenticationError as response code to creation
 * Send creation reponse code (distinguish incorrect params from creation error).
 * Wrap the Engine process in a ProcessHandle.  Try 3 times to start the webx-engine (in case Xorg is slow to start).
 * Improve logging
 * Remove engine session container: use mutex protected Vec of EngineSessions in EngineSessionManager.
 * Refactor engine_connector to engine_communicator and attach to engine struct (one per engine): keep socket open for duration of engine life.
 * Refactoring engine related files to engine module (remove service module).
 * Move config to an example file. add config.yml to gitignore.
 * Separate Engine creation/communication from EngineSession management.
 * Merge of webx-session-manager into webx-router: creation/management of Xorg and Window manager processes handled here rather than making zmq requests to the session manager.

> Note that from this version the [WebX Session Manager](https://github.com/ILLGrenoble/webx-session-manager) is not longer necessary.

1.3.0 07/05/2025
================
 * Remove specific limit on number of parameters expected for session commands (allow for future additional parameters)

1.2.0 25/04/2025
================
 * Pass engine parameters in session creation command. These are converted into environment variables when spawning an engine.

1.1.0 11/04/2025
================
 * Modify default config so that sessions aren't deleted on inactivity.
 * fix version of debian in Dockerfile.debian (maintain compatibility with local ci/cd server)

1.0.0 01/04/2025
================
 * Build releases for Ubuntu 20 and arm architecture
 * Full code documentation
 * Enable 15s timeout when requesting a session from the session manager.
 * Add optional logging to file.
 * Forward connect and disconnect commands to session proxy. 
 * Add a connector class to send reqrep commands the engine.

0.1.4 11/02/2025
================
 * Build releases versions also for Debian 12 and Ubuntu 24
 * Update dependencies

0.1.3 28/01/2025
================
 * Update README
 * Add BSD license

0.1.2 02/01/2025
================
 * Set the package version in the ci/cd automatically to the tag value

0.1.1 19/12/2024
================
* Add github action to build and upload debian packages to release files

0.1.0 17/02/2023
================
 * Initial release

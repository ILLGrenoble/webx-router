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

# Potato Launcher

## Environment Variables

The following environment variables are required to build the launcher:

- `LAUNCHER_NAME`: The name of the launcher.
- `SERVER_BASE`: The base URL of the server.
- `DISPLAY_LAUNCHER_NAME` (optional): The display name of the launcher. Defaults to `LAUNCHER_NAME` if not provided.
- `TGAUTH_BASE` (optional): The base URL for TGAuth.
- `ELYBY_APP_NAME` (optional): The Ely.by application name.
- `ELYBY_CLIENT_ID` (optional): The Ely.by client ID.
- `ELYBY_CLIENT_SECRET` (optional): The Ely.by client secret.

> **Note:** Either `TGAUTH_BASE` or all of the `ELYBY_APP_NAME`, `ELYBY_CLIENT_ID`, and `ELYBY_CLIENT_SECRET` must be present to build the launcher successfully.

The following environment variables are additionally used in the CI/CD workflow:

- `VERSION` (optional): Launcher version. Is used to compare to the remove version and update if necessary.
- `SSH_KEY`: The SSH key for deploying to the server.
- `SERVER_USER`: The username for the server.
- `SERVER_ADDR`: The address of the server.
- `SERVER_PATH`: The path on the server where the launcher will be deployed.

## Building the Launcher on Linux

To build the launcher on a Linux machine, follow these steps:

1. **Clone the Repository:**
   ```bash
   git clone <repository-url>
   cd <repository-directory>
   ```

2. **Install Rust:**
   If you don't have Rust installed, install it using `rustup`:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **Set Up Environment Variables:**
   Ensure the necessary environment variables are set.

   Example:
   ```bash
   export LAUNCHER_NAME="potato_launcher"
   export SERVER_BASE="https://example.com"
   export TGAUTH_BASE="https://example.com"
   export DISPLAY_LAUNCHER_NAME="Potato Launcher"
   ```

4. **Build the Launcher:**
   Run the following command to build the launcher:
   ```bash
   cargo build --release
   ```

   To create an app bundle on macOS, you can install `cargo-bundle` tool:
   ```bash
   cargo install cargo-bundle
   cargo bundle --release
   ```

   If you want to distribute the app bundle, you need to sign it:
   ```bash
   codesign --force --deep --sign - "target/release/bundle/osx/<your app name>.app"
   ```

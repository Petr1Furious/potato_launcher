# Potato Launcher

A Minecraft launcher that is both extremely easy to use and trivial to deploy with basic DevOps skills. Perfect for frequently changing instances.

## Key Features

- **Simple UI**: Does all the hard work in the background, including syncing instance files, installing Java, and self-updating.
- **Instance Builder Tool**: Comes with [a tool](instance_builder/) to easily create and deploy different versions and instances in a conformant format (which is an extension to the vanilla one). The tool can make the launcher sync arbitrary files, including mods and configs, with or without overwriting them if they exist, and requires minimal setup.
- **Performance**: _blazinglyfast_, not just because it is written in Rust, but also because it is async and multithreaded. It syncs only the missing/changed files, minimizing network requests as much as possible. If the instance hasn't changed, it won't make any extra network requests at all.
- **Custom Authorization**: Supports custom authorization servers.
- **Vanilla format**: Fully compatible with vanilla, Forge, and Fabric version metadata formats. It can even be built with [vanilla manifest](https://piston-meta.mojang.com/mc/game/version_manifest_v2.json) and launch vanilla versions out of the box, just like the vanilla launcher.

## Building from Source

### Environment Variables

The following environment variables are required to build the launcher:

- `LAUNCHER_NAME`: The name of the launcher, e.g., "Potato Launcher".
- `VERSION_MANIFEST_URL`: URL pointing to the version manifest, e.g., `https://piston-meta.mojang.com/mc/game/version_manifest_v2.json`.

### Building with Rust

To build the launcher on Linux/MacOS, follow these steps:

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
   export LAUNCHER_NAME="Potato Launcher"
   export VERSION_MANIFEST_URL="https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"
   ```

4. **Build the Launcher:**
   Run the following command to build the launcher:
   ```bash
   cargo build --bin launcher --release
   ```

   To create an app bundle on macOS, you can install the `cargo-bundle` tool:
   ```bash
   cargo install cargo-bundle
   cargo bundle --bin launcher --release
   ```

   If you want to distribute the app bundle, you need to sign it:
   ```bash
   codesign --force --deep --sign - "target/release/bundle/osx/<your app name>.app"
   ```

## Creating Instances for Backend

See [instance builder](instance_builder/).

## Setting up GitHub Actions

To deploy new versions of the launcher automatically, set the following secrets and variables:

- `VERSION`: Launcher version, set automatically in the workflow. Used to compare with the remote version and update if necessary.
- `AUTO_UPDATE_BASE`: The URL that will store launcher update files.
- `SSH_KEY`: The SSH key for deploying to the server.
- `SERVER_USER`: The username for the server.
- `SERVER_ADDR`: The address of the server.
- `SERVER_PATH`: The path on the server where the launcher binaries will be deployed. The files have to be accessible by `AUTO_UPDATE_BASE/<binary name>`.

See the [workflow file](.github/workflows/deploy.yml) for more details.

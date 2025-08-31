# Server configuration

::: info Requirements
- A Linux server with a public IP address
- A domain name
- Basic Linux skills
:::

You need to configure the server to serve files from a directory under your domain name. This directory will contain all the launcher files, including launcher binaries, modpacks and various metadata. Launcher binaries will be stored in the `launcher` subdirectory, and modpack files will live in `data`.

For example, if your directory is `/srv/potatosmp` and your domain is `mc.potato.smp`, visiting `https://mc.potato.smp/data/version_manifest.json` should return the `/srv/potatosmp/data/version_manifest.json` file.

You can achieve this with a nginx config like this:

```nginx
server {
    <...>
    server_name mc.potato.smp;
    location / {
        index index.html;
        root /srv/potatosmp;
        autoindex on;
    }
}
```

::: warning
While it may be possible to use plain HTTP without a TLS certificate, it's not tested and not supported. Please don't do that.
:::

## User creation

You also need to allow the GitHub Actions pipeline to copy new launcher versions to this directory over SSH. The safest way to do that is to create a new user. I recommend configuring permissions as follows:

1. Create a matching group and user. For example, `potatosmp`. You can use the following command:
   ```bash
   useradd -U -m -s /bin/bash potatosmp
   ```
2. Change permissions of your launcher directory:
   ```bash
   chown -R potatosmp:potatosmp /srv/potatosmp
   # Make sure permissions are properly set
   chmod -R u+rwx,g+rx /srv/potatosmp
   ```
3. Add other users that need access to the newly created group. You'll definitely want to add your nginx user, and possibly your main user, if you want to use it to upload modpacks.
   ```bash
   usermod -aG potatosmp nginx
   ```
   Note that the nginx user can be different in different distros. Look at the top of your `nginx.conf` or in the nginx systemd service to find your user name
4. Log in as your user and generate an SSH key
   ```bash
   sudo -iu potatosmp
   ssh-keygen -t ed25519
   cat ~/.ssh/id_ed25519.pub >> ~/.ssh/authorized_keys
   cat ~/.ssh/id_ed25519  # Copy the key and save it for the next step
   ```

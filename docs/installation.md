# 🚀 Installation & Running

## Prerequisites

**Gitwig** requires the following external tools to be installed on your system:
- **`git`**: A system installation of `git` is required in your `PATH` for operations like staging, diffing, pushing, pulling, and network syncing.
- **`ssh`**: Required if you use SSH-based Git remotes for network operations.
  > [!NOTE]
  > **SSH Host Key Policy:** To prevent connection hangs in the non-interactive TUI context, Gitwig executes git commands with `GIT_SSH_COMMAND` set to `ssh -o StrictHostKeyChecking=accept-new`. This automatically trusts and saves new host keys, while verifying existing ones to protect against MITM attacks. If you require a different SSH policy, make sure to add host keys to your `known_hosts` beforehand.


### Via Homebrew (macOS / Linux)

You can tap and install **Gitwig** using Homebrew:

```sh
brew tap tareqmy/gitwig
brew install gitwig
```

*Note: If Homebrew prompts you with an "untrusted tap" error, run `brew trust tareqmy/gitwig` to trust it, and then run `brew install gitwig` again.*

### Via Shell Script (macOS / Linux)

For a quick installation, run the following command in your terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/tareqmy/gitwig/master/scripts/install.sh | sh
```

### On Windows

You can install **Gitwig** on Windows via PowerShell:

Run the following command in PowerShell:

```powershell
irm https://raw.githubusercontent.com/tareqmy/gitwig/master/scripts/install.ps1 | iex
```
* **Benefits**: Quick command-line installation to your user profile directory (`%USERPROFILE%\.gitwig\bin`) without requiring administrator privileges.
* **Updating**: Supports the in-app self-updater, which will automatically run `install.ps1` to download newer versions.



### Via Cargo

You can install **Gitwig** directly from [crates.io](https://crates.io/crates/gitwig):

```sh
cargo install gitwig
```

### Building from Source

Alternatively, you can clone the repository and build it from source:

```sh
git clone https://github.com/tareqmy/gitwig.git
cd gitwig
cargo build --release
```

The compiled binary will be located at `target/release/gitwig`. You can copy it to a directory in your `$PATH` or run it directly.

### Uninstalling

If you installed Gitwig via the shell script, you can uninstall it at any time using:

```sh
curl -fsSL https://raw.githubusercontent.com/tareqmy/gitwig/master/scripts/uninstall.sh | sh
```

If you installed Gitwig via the PowerShell script, you can uninstall it at any time using:

```powershell
irm https://raw.githubusercontent.com/tareqmy/gitwig/master/scripts/uninstall.ps1 | iex
```


### Running

You can launch **Gitwig** using either the full name or the short command alias `gtg`:

```sh
# Run with default config resolution
gitwig
# Or using the shortcut:
gtg

# Run with an explicit config path
gitwig path/to/config.toml
# Or:
gtg path/to/config.toml
```

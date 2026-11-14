# Snowhush Portable

An unofficial portable build of [Snowhush](https://github.com/mbezruchko/Snowhush) for Windows, based on v0.13.1.
It is not made by or affiliated with the original author.

Snowhush puts soft snow on your desktop. This build changes only how it stores things, so it can run from a folder or a USB stick without touching the system.

## What is different

- **No registry writes.** Settings are kept in `Snowhush.settings` beside the exe. (The registry is only read, for the Windows theme and accent colour.)
- **No "Open at Login"** entry. Make a shortcut in `shell:startup` yourself if you want it.
- **No `--uninstall` mode.** Delete the folder to remove it.
- **No log file** unless you start it with `--log` (then `snowhush.log` is written beside the exe).

Unchanged: the update check still reads from api.github.com, and only one copy can run at a time.

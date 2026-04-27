# Unity LS

A Language Server Protocol (LSP) implementation focused on Unity-specific features for C# scripts.

## Features

- **Asset References**: Shows where a script component is referenced in scenes (.unity), prefabs (.prefab), and assets (.asset) files using code lens

## Building

```bash
cargo build --release
```

The binary will be located at `target/release/unity_ls`.

## Running

The server uses stdio-based communication. Start it with your LSP client (e.g., Visual Studio Code):

```bash
./target/release/unity_ls
```

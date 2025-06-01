# ASCII FaceTime

A lightweight video chat app that turns webcam streams into ASCII art and sends them over the network. It uses a TCP/UDP SFU (Selective Forwarding Unit) server to connect users in real time—essentially FaceTime, but in terminal-friendly ASCII.

## Features

- ASCII video streaming from webcam
- Peer-to-peer connection via TCP/UDP SFU server
- Simple CLI-based interface
- Built in Rust with OpenCV and LLVM bindings

## Build Instructions (macOS)

To build the client on macOS, run:

```bash
./build-client-macos.sh
```
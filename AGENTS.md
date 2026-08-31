# CMDR

Cmdr is a small desktop app written in Tauri (React + Typescript), which allows you to manage and run "layers" of processes through a GUI.
Cmdr does not configure any processes, but is simply a control plane.

## Layers

In Cmdr, a "layer" is a collection of processes that can be started, stopped, and managed together. 
A layer is either of type "docker" or "terminal", meaning it's either a docker container or a separately running process, typically via shell command.

# Keybindings

Every key and mouse action in VoidLink, and how to remap them. Press <kbd>?</kbd> in the app for the same list, drawn from your current bindings.

## Keys

| Key | Action | Config name |
| --- | --- | --- |
| <kbd>j</kbd> <kbd>k</kbd> / <kbd>↓</kbd> <kbd>↑</kbd> | Move down / up | `nav_down`, `nav_up` |
| <kbd>g</kbd> <kbd>G</kbd> | Jump to top / bottom | `jump_top`, `jump_bottom` |
| <kbd>/</kbd> | Search (start the query with `/` for regex) | `search` |
| <kbd>S</kbd> | Cycle sort: smart, name, signal, address | `cycle_sort` |
| <kbd>Esc</kbd> | Clear search, close a dialog | |
| <kbd>Enter</kbd> | Connect or disconnect; pairs first if needed | `connect_toggle` |
| <kbd>d</kbd> | Disconnect | `disconnect` |
| <kbd>p</kbd> | Pair only | `pair` |
| <kbd>t</kbd> | Trust / untrust (trusted devices reconnect on their own) | `trust` |
| <kbd>A</kbd> | Rename | `rename` |
| <kbd>r</kbd> | Forget the device (asks first) | `remove` |
| <kbd>R</kbd> | Refresh the device | `refresh` |
| <kbd>s</kbd> | Start / stop scanning | `toggle_scan` |
| <kbd>a</kbd> | Turn Bluetooth on / off | `toggle_adapter` |
| <kbd>?</kbd> | Help | `help` |
| <kbd>q</kbd> / <kbd>Ctrl</kbd>+<kbd>C</kbd> | Quit | `quit` |

In a pairing prompt, <kbd>y</kbd> accepts and <kbd>n</kbd> rejects.

## Mouse

Click to select, double-click to connect, scroll to move. Turn it off with `--no-mouse` or `mouse = false`.

## Remapping

Set any action under `[keybindings]` in `~/.config/voidlink/config.toml`:

```toml
[keybindings]
connect_toggle = "Space"
toggle_scan = "S"
cycle_sort = "o"
```

- A key is one character (`"q"`, `"/"`, `"G"`) or a named key: `Enter`, `Esc`, `Tab`, `Space`, `Backspace`, `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PageUp`, `PageDown`, `Delete`, `Insert`.
- Arrow keys always navigate, and <kbd>Ctrl</kbd>+<kbd>C</kbd> always quits.
- An unknown key name, or two actions sharing a key, is reported in the app.

[Back to the README](../README.md)

# VoidLink demo, rendered with https://github.com/charmbracelet/vhs
#
#   cargo build --release --features demo --target-dir target/demo
#   vhs demo/voidlink.tape        # run from the repo root; writes assets/demo.mp4
#
# Uses the scripted demo adapter (VOIDLINK_DEMO=1) and a home that doesn't
# exist, so no real devices, addresses or paths appear in the recording.

Output assets/demo.mp4

Require vhs

Set Shell "bash"
Set FontFamily "JetBrainsMono Nerd Font Mono"
Set FontSize 36
Set LineHeight 1.2
Set Width 2560
Set Height 1600
Set Padding 36
Set Margin 56
Set MarginFill "#120b24"
Set BorderRadius 16
Set WindowBar Colorful
Set WindowBarSize 48
Set Framerate 25
Set TypingSpeed 90ms
Set CursorBlink false
Set Theme { "name": "Void", "background": "#0b0d17", "foreground": "#e8e6f0", "cursor": "#00e5ff", "selection": "#2a2f4a", "black": "#0b0d17", "brightBlack": "#4a4e69", "red": "#ff4545", "brightRed": "#ff6b6b", "green": "#69f0ae", "brightGreen": "#69f0ae", "yellow": "#ffb74d", "brightYellow": "#ffd180", "blue": "#00e5ff", "brightBlue": "#18ffff", "magenta": "#bb86fc", "brightMagenta": "#d0a8ff", "cyan": "#00e5ff", "brightCyan": "#18ffff", "white": "#e8e6f0", "brightWhite": "#ffffff" }

# Launch off camera so the first frame is the populated app, not a prompt.
Hide
Type "clear && HOME=/home/demo XDG_CONFIG_HOME=/home/demo/.config XDG_STATE_HOME=/home/demo/.local/state VOIDLINK_DEMO=1 target/demo/release/voidlink"
Enter
Sleep 1.2s
Show

# Beat 1: your devices, with live status, battery and signal
Sleep 2.4s

# Beat 2: scan, and nearby devices stream in
Type "s"
Sleep 3.6s

# Beat 3: find one with search
Type "/"
Sleep 0.4s
Type@140ms "jbl"
Sleep 0.6s
Enter
Sleep 0.8s

# Beat 4: connect; VoidLink pairs first and asks you to confirm the code
Enter
Sleep 2.6s
Type "y"
Sleep 2.8s
Escape
Sleep 0.8s

# Beat 5: give it a friendlier name
Type "A"
Sleep 0.5s
Ctrl+U
Type@100ms "Living Room"
Sleep 0.5s
Enter
Sleep 2s

# Beat 6: every key, one keypress away
Type "s"
Sleep 0.6s
Type "?"
Sleep 2.8s

# Close where it opened so the loop is seamless.
Escape
Sleep 1.5s

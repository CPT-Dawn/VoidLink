# VoidLink demo — rendered with https://github.com/charmbracelet/vhs
#
#   cargo build --release --features demo --target-dir target/demo
#   vhs demo/voidlink.tape
#
# Uses the scripted demo adapter (VOIDLINK_DEMO=1), so no real devices or
# MAC addresses appear in the recording.

Output assets/demo.mp4

Require vhs

Set Shell "bash"
Set FontFamily "JetBrainsMono Nerd Font Mono"
Set FontSize 28
Set LineHeight 1.25
Set Width 2560
Set Height 1440
Set Padding 36
Set Margin 56
Set MarginFill "#120b24"
Set BorderRadius 16
Set WindowBar Colorful
Set WindowBarSize 48
Set Framerate 30
Set TypingSpeed 90ms
Set CursorBlink false
Set Theme { "name": "Void", "background": "#0b0d17", "foreground": "#e8e6f0", "cursor": "#00e5ff", "selection": "#2a2f4a", "black": "#0b0d17", "brightBlack": "#4a4e69", "red": "#ff4545", "brightRed": "#ff6b6b", "green": "#69f0ae", "brightGreen": "#69f0ae", "yellow": "#ffb74d", "brightYellow": "#ffd180", "blue": "#00e5ff", "brightBlue": "#18ffff", "magenta": "#bb86fc", "brightMagenta": "#d0a8ff", "cyan": "#00e5ff", "brightCyan": "#18ffff", "white": "#e8e6f0", "brightWhite": "#ffffff" }

Env VOIDLINK_DEMO "1"

Hide
Type "clear && target/demo/release/voidlink"
Enter
Sleep 1.2s
Show

# Your devices, with live status and battery
Sleep 2.2s
Type "j"
Sleep 1.1s
Type "j"
Sleep 1.1s
Type "g"
Sleep 0.8s

# Scan for nearby devices
Type "s"
Sleep 4.6s

# Find one with search
Type "/"
Sleep 0.5s
Type@140ms "jbl"
Sleep 0.8s
Enter
Sleep 1s

# Connect: VoidLink pairs first and asks you to confirm the code
Enter
Sleep 3s
Type "y"
Sleep 3.2s

# Back to the full list
Escape
Sleep 1.2s

# Give it a friendlier name
Type "A"
Sleep 0.6s
Ctrl+U
Type@110ms "Living Room"
Sleep 0.6s
Enter
Sleep 2.4s

# Stop scanning, peek at the help
Type "s"
Sleep 1s
Type "?"
Sleep 3.4s
Escape
Sleep 1.2s
Type "q"
Sleep 0.5s

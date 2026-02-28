# VHS tape file for recording nv-swaptop demo
# Install: https://github.com/charmbracelet/vhs
# Usage: vhs docs/record.tape

Output docs/swaptop-demo.gif

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 700
Set Padding 10
Set Framerate 15
Set PlaybackSpeed 0.5
Set TypingSpeed 0ms

# Launch nv-swaptop
Type "cargo run --release"
Enter
Sleep 3s

# Tab 1: Swap View (default)
Sleep 3s

# Tab 2: NUMA Topology View
Type "2"
Sleep 3s

# Tab 3: GPU View
Type "3"
Sleep 3s

# Tab 4: Unified CPU+GPU+NUMA View
Type "4"
Sleep 3s

# Cycle back to show scrolling on swap view
Type "1"
Sleep 1s
Down
Down
Down
Sleep 2s

# Show theme cycling
Type "t"
Sleep 1.5s
Type "t"
Sleep 1.5s
Type "t"
Sleep 1.5s
Type "t"
Sleep 1.5s
Type "t"
Sleep 1.5s

# Quit
Type "q"
Sleep 1s

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

# Launch nv-swaptop in demo mode (auto-cycles all 4 views, then quits)
Type "./target/release/nv-swaptop --demo"
Enter
Sleep 18s

# VHS tape for docs/assets/demo.gif
# Generate with: vhs docs/assets/demo.tape
# Requires vhs installed; do not link a missing gif from README.

Output docs/assets/demo.gif
Set FontSize 14
Set Width 1000
Set Height 600
Set TypingSpeed 35ms

Hide
Type "cargo run -- --demo --read-only --ascii"
Enter
Sleep 2s
Show

# Dashboard
Sleep 900ms
# Processes + select
Type "2"
Sleep 700ms
Type "j"
Sleep 200ms
Type "j"
Sleep 400ms
Type "Enter"
Sleep 800ms
Type "Esc"
Sleep 400ms
# Services
Type "3"
Sleep 800ms
# Logs
Type "4"
Sleep 800ms
# Diagnostics
Type "6"
Sleep 1000ms
# Back to dashboard then quit
Type "1"
Sleep 600ms
Type "q"

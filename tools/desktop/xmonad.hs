import XMonad
import XMonad.Hooks.EwmhDesktops (ewmh, ewmhFullscreen)

main :: IO ()
main = xmonad $ ewmhFullscreen $ ewmh def

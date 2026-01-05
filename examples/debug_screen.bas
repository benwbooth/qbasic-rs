' Debug screen dimensions
PRINT "SCREENWIDTH:"; SCREENWIDTH
PRINT "SCREENHEIGHT:"; SCREENHEIGHT

' Draw a red border around the entire screen
scrWidth = SCREENWIDTH
scrHeight = SCREENHEIGHT

COLOR 4
' Top line
LINE (0, 0)-(scrWidth - 1, 0)
' Bottom line
LINE (0, scrHeight - 1)-(scrWidth - 1, scrHeight - 1)
' Left line
LINE (0, 0)-(0, scrHeight - 1)
' Right line
LINE (scrWidth - 1, 0)-(scrWidth - 1, scrHeight - 1)

' Draw a green filled box in the center
cx = scrWidth / 2
cy = scrHeight / 2
boxSize = 50
COLOR 2
LINE (cx - boxSize, cy - boxSize)-(cx + boxSize, cy + boxSize), , BF

' Draw a yellow circle
COLOR 14
CIRCLE (cx, cy), boxSize

' Print dimensions again at top
LOCATE 1, 1
COLOR 15
PRINT "W:"; scrWidth; "H:"; scrHeight

INPUT "Press Enter to exit: ", dummy

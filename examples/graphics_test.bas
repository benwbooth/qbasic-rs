' Simple graphics test
SCREEN 12  ' VGA 640x480 16-color mode

' Draw a red circle
COLOR 4
CIRCLE (320, 240), 100

' Draw a blue line
COLOR 1
LINE (0, 0)-(640, 480)

' Draw a green filled box
COLOR 2
LINE (100, 100)-(200, 200), , BF

' Draw some points
COLOR 15
FOR i = 0 TO 100
    PSET (50 + i, 50)
NEXT i

' Program ends - you'll see "Press any key to continue"
END

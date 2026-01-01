' Graphics Demo - Tests sixel rendering
' This program draws various graphics primitives

SCREEN 12  ' VGA 640x480 16-color mode

' Draw colorful background gradient (vertical stripes)
FOR c = 0 TO 15
    COLOR c
    x1 = c * 40
    LINE (x1, 0)-(x1 + 40, 480), , BF
NEXT c

' Draw a large white circle in the center
COLOR 15
CIRCLE (320, 240), 150

' Fill it with yellow
COLOR 14
PAINT (320, 240), 14

' Draw concentric circles
FOR r = 20 TO 140 STEP 20
    COLOR (r / 20) MOD 16
    CIRCLE (320, 240), r
NEXT r

' Draw some lines radiating from center
FOR angle = 0 TO 360 STEP 30
    pi = 3.14159
    radAngle = angle * pi / 180
    x2 = 320 + 200 * COS(radAngle)
    y2 = 240 + 200 * SIN(radAngle)
    COLOR angle / 30
    LINE (320, 240)-(x2, y2)
NEXT angle

' Draw boxes in corners
COLOR 4
LINE (10, 10)-(100, 80), , BF

COLOR 2
LINE (530, 10)-(630, 80), , BF

COLOR 1
LINE (10, 390)-(100, 470), , BF

COLOR 5
LINE (530, 390)-(630, 470), , BF

' Draw some individual pixels spelling "QBASIC"
COLOR 15
' Q
PSET (50, 120)
PSET (51, 120)
PSET (52, 120)
PSET (49, 121)
PSET (53, 121)
PSET (49, 122)
PSET (53, 122)
PSET (49, 123)
PSET (51, 123)
PSET (53, 123)
PSET (50, 124)
PSET (51, 124)
PSET (53, 124)

END

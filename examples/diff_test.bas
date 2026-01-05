' Differential sixel update test
' Draws a background, then updates a small area with circles

scrWidth = SCREENWIDTH
scrHeight = SCREENHEIGHT

PRINT "Screen:"; scrWidth; "x"; scrHeight

' Fill background with blue
COLOR 1
LINE (0, 0)-(scrWidth, scrHeight), , BF

' Draw a yellow sun (circle) in top right
COLOR 14
CIRCLE (scrWidth - 100, 100), 50

' Draw initial red ball (circle)
ballX = 130
ballY = scrHeight / 2
ballRadius = 30
COLOR 4
CIRCLE (ballX, ballY), ballRadius

PRINT "Ball at"; ballX; ","; ballY
PRINT "Press Enter to move ball..."
INPUT "", x

' Move ball - should only update the ball region, not redraw whole screen
FOR i = 1 TO 10
    ' Erase old ball (with blue circle)
    COLOR 1
    CIRCLE (ballX, ballY), ballRadius

    ' Move ball
    ballX = ballX + 80

    ' Draw new ball (red circle)
    COLOR 4
    CIRCLE (ballX, ballY), ballRadius

    ' Small delay
    FOR j = 1 TO 50000
    NEXT j
NEXT i

PRINT ""
PRINT "Ball moved to"; ballX
INPUT "Done: ", x

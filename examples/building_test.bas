' Minimal test for LINE...BF with FOR loop
DIM buildingX(5)
DIM buildingW(5)
DIM buildingH(5)

scrWidth = SCREENWIDTH
scrHeight = SCREENHEIGHT

PRINT "Screen:"; scrWidth; "x"; scrHeight

' Setup building data
buildingX(0) = 0
buildingW(0) = 100
buildingH(0) = 200

buildingX(1) = 120
buildingW(1) = 100
buildingH(1) = 250

buildingX(2) = 240
buildingW(2) = 100
buildingH(2) = 180

' Fill blue sky
COLOR 1
LINE (0, 0)-(scrWidth, scrHeight), , BF
PRINT "Blue sky drawn"

' Draw sun (to verify CIRCLE works)
COLOR 14
CIRCLE (scrWidth - 80, 80), 40
PAINT (scrWidth - 80, 80), 14
PRINT "Sun drawn"

' Draw buildings with explicit FOR loop
PRINT "Drawing 3 buildings..."
FOR i = 0 TO 2
    bx = buildingX(i)
    bw = buildingW(i)
    bh = buildingH(i)

    PRINT "Building"; i; ": x="; bx; " w="; bw; " h="; bh

    ' Draw building (gray)
    COLOR 8
    LINE (bx, scrHeight - bh)-(bx + bw, scrHeight), , BF
NEXT i

PRINT "Done drawing buildings"
INPUT "Press Enter to exit: ", x

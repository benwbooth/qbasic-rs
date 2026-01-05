' Debug version - just test the building drawing

DIM buildingX(15)
DIM buildingW(15)
DIM buildingH(15)

scrWidth = SCREENWIDTH
scrHeight = SCREENHEIGHT
numBuildings = 9

PRINT "Screen:"; scrWidth; "x"; scrHeight
PRINT "numBuildings:"; numBuildings

' Generate buildings
buildingBaseW = scrWidth / numBuildings
PRINT "buildingBaseW:"; buildingBaseW
currentX = 0

FOR i = 0 TO numBuildings - 1
    buildingW(i) = buildingBaseW * 0.7 + INT(RND * buildingBaseW * 0.3)
    buildingH(i) = scrHeight * 0.2 + INT(RND * scrHeight * 0.4)
    buildingX(i) = currentX
    currentX = currentX + buildingW(i)
    PRINT "Building"; i; ": x="; buildingX(i); " w="; buildingW(i); " h="; buildingH(i)
NEXT i

PRINT ""
PRINT "Now drawing..."

' Clear with blue
COLOR 1
LINE (0, 0)-(scrWidth, scrHeight), , BF

' Draw buildings
FOR i = 0 TO numBuildings - 1
    bx = buildingX(i)
    bh = buildingH(i)
    bw = buildingW(i)

    PRINT "Draw building"; i; ": ("; bx; ","; scrHeight - bh; ")-("; bx + bw; ","; scrHeight; ")"

    COLOR 8
    LINE (bx, scrHeight - bh)-(bx + bw, scrHeight), , BF
NEXT i

INPUT "Done: ", x

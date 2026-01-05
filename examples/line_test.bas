' Test LINE...BF with explicit colors
scrWidth = SCREENWIDTH
scrHeight = SCREENHEIGHT

PRINT "Testing LINE...BF"
PRINT "Screen:"; scrWidth; "x"; scrHeight

' Fill blue background
COLOR 1
LINE (0, 0)-(scrWidth, scrHeight), , BF
PRINT "Blue background drawn"

' Draw gray box in center
COLOR 8
LINE (100, 100)-(300, 300), , BF
PRINT "Gray box (color 8) at 100,100"

' Draw yellow box
COLOR 14
LINE (350, 100)-(550, 300), , BF
PRINT "Yellow box (color 14) at 350,100"

' Draw red box
COLOR 4
LINE (100, 350)-(300, 550), , BF
PRINT "Red box (color 4) at 100,350"

INPUT "Done - press Enter: ", x

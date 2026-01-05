' Simple graphics test
PRINT "Screen size:"; SCREENWIDTH; "x"; SCREENHEIGHT
COLOR 4
LINE (10, 10)-(100, 100), , BF
COLOR 14
LINE (50, 50)-(150, 150), , BF
PRINT "Drew two boxes"
INPUT "Press Enter to exit: ", x$

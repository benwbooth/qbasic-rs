' Array test
DIM arr(5)

PRINT "Setting array values"
FOR i = 0 TO 5
    arr(i) = i * 10
    PRINT "Set arr("; i; ") ="; i * 10
NEXT i

PRINT ""
PRINT "Reading array values"
FOR i = 0 TO 5
    PRINT "arr("; i; ") ="; arr(i)
NEXT i

INPUT "Done: ", x

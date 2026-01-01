CLS
PRINT "Testing WHILE loop and arrays"

DIM arr(10)
arr(0) = 5
arr(1) = 10

gameOver = 0
counter = 0

WHILE gameOver = 0
    PRINT "Loop iteration:"; counter
    counter = counter + 1

    IF counter >= 3 THEN
        gameOver = 1
    END IF
WEND

PRINT "Loop finished!"
PRINT "Array test: arr(0) ="; arr(0); " arr(1) ="; arr(1)
INPUT "Press Enter: ", x
END

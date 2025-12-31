' NIBBLES DEBUG VERSION - prints collision info

' Game constants
MAXLEN = 500
WIDTH = 78
HEIGHT = 22

' Snake body arrays
DIM snakeX(500)
DIM snakeY(500)

' === GAME START ===
100 CLS
COLOR 14, 1
PRINT "NIBBLES DEBUG - Press key to start"

' Wait for key
110 k$ = INKEY$
IF k$ = "" THEN GOTO 110

' Initialize game
RANDOMIZE TIMER
score = 0
snakeLen = 5
direction = 4
gameOver = 0
speed = 100
moveCount = 0

' Initial snake position (middle of screen)
startX = WIDTH / 2
startY = HEIGHT / 2
FOR i = 0 TO snakeLen - 1
    snakeX(i) = startX - i
    snakeY(i) = startY
NEXT i

' Place first food
GOSUB 500

' Draw initial screen
GOSUB 600
GOSUB 700
GOSUB 800
GOSUB 900

' Print debug info
LOCATE 24, 1
COLOR 15, 0
PRINT "Snake head: "; snakeX(0); ","; snakeY(0); " Food: "; foodX; ","; foodY; "   ";

' === MAIN GAME LOOP ===
lastMove = TIMER
200 k$ = INKEY$
IF k$ <> "" THEN
    IF LEN(k$) >= 1 THEN
        c = ASC(k$)
        ' Check for arrow keys (escape sequences)
        IF c = 27 AND LEN(k$) >= 3 THEN
            arrow$ = RIGHT$(k$, 1)
            IF arrow$ = "A" AND direction <> 2 THEN direction = 1
            IF arrow$ = "B" AND direction <> 1 THEN direction = 2
            IF arrow$ = "D" AND direction <> 4 THEN direction = 3
            IF arrow$ = "C" AND direction <> 3 THEN direction = 4
        END IF
        ' WASD keys
        IF c = 119 OR c = 87 THEN
            IF direction <> 2 THEN direction = 1
        END IF
        IF c = 115 OR c = 83 THEN
            IF direction <> 1 THEN direction = 2
        END IF
        IF c = 97 OR c = 65 THEN
            IF direction <> 4 THEN direction = 3
        END IF
        IF c = 100 OR c = 68 THEN
            IF direction <> 3 THEN direction = 4
        END IF
        ' Quit on Q
        IF c = 113 OR c = 81 THEN
            IF LEN(k$) = 1 THEN gameOver = 2
        END IF
    END IF
END IF

' Move snake at regular intervals
currentTime = TIMER
IF currentTime - lastMove >= speed / 1000 THEN
    lastMove = currentTime
    moveCount = moveCount + 1
    GOSUB 400
    ' Debug: show head and food positions
    LOCATE 24, 1
    COLOR 15, 0
    PRINT "Move "; moveCount; ": Head "; snakeX(0); ","; snakeY(0); " Food "; foodX; ","; foodY; " Score "; score; "   ";
    IF gameOver = 1 THEN GOTO 300
END IF

IF gameOver = 0 THEN GOTO 200

' === GAME OVER SCREEN ===
300 COLOR 15, 4
LOCATE 12, 30
PRINT " GAME OVER! "
COLOR 7, 1
LOCATE 14, 25
PRINT "Your score: "; score
LOCATE 16, 22
PRINT "Press R to restart, Q to quit"

310 k$ = INKEY$
IF k$ = "" THEN GOTO 310
c = ASC(k$)
IF c = 114 OR c = 82 THEN GOTO 100
IF c = 113 OR c = 81 THEN
    CLS
    END
END IF
GOTO 310

' === MOVE SNAKE SUBROUTINE ===
400 headX = snakeX(0)
headY = snakeY(0)

IF direction = 1 THEN headY = headY - 1
IF direction = 2 THEN headY = headY + 1
IF direction = 3 THEN headX = headX - 1
IF direction = 4 THEN headX = headX + 1

' Check wall collision
IF headX < 2 OR headX > WIDTH - 1 THEN
    gameOver = 1
    RETURN
END IF
IF headY < 2 OR headY > HEIGHT - 1 THEN
    gameOver = 1
    RETURN
END IF

' Check self collision
FOR i = 0 TO snakeLen - 1
    IF headX = snakeX(i) AND headY = snakeY(i) THEN
        gameOver = 1
        RETURN
    END IF
NEXT i

' Check food collision - DEBUG
ateFood = 0
IF headX = foodX AND headY = foodY THEN
    ateFood = 1
    score = score + 10
    snakeLen = snakeLen + 1
    IF snakeLen > MAXLEN - 1 THEN snakeLen = MAXLEN - 1
    IF speed > 50 THEN speed = speed - 1
    GOSUB 500
    GOSUB 800
    GOSUB 900
END IF

' Erase tail (unless we ate food)
IF ateFood = 0 THEN
    tailX = snakeX(snakeLen - 1)
    tailY = snakeY(snakeLen - 1)
    LOCATE tailY, tailX
    COLOR 7, 1
    PRINT " ";
END IF

' Move body segments
FOR i = snakeLen - 1 TO 1 STEP -1
    snakeX(i) = snakeX(i - 1)
    snakeY(i) = snakeY(i - 1)
NEXT i

' Set new head position
snakeX(0) = headX
snakeY(0) = headY

' Draw new head
LOCATE headY, headX
COLOR 10, 1
PRINT CHR$(219);

RETURN

' === PLACE FOOD SUBROUTINE ===
500 validFood = 0
WHILE validFood = 0
    foodX = INT(RND * (WIDTH - 4)) + 2
    foodY = INT(RND * (HEIGHT - 4)) + 2
    validFood = 1
    ' Make sure food isn't on snake
    FOR i = 0 TO snakeLen - 1
        IF foodX = snakeX(i) AND foodY = snakeY(i) THEN
            validFood = 0
        END IF
    NEXT i
WEND
RETURN

' === DRAW BORDER SUBROUTINE ===
600 CLS
COLOR 15, 1

' Top border
LOCATE 1, 1
PRINT CHR$(201);
FOR i = 2 TO WIDTH - 1
    PRINT CHR$(205);
NEXT i
PRINT CHR$(187);

' Side borders
FOR row = 2 TO HEIGHT - 1
    LOCATE row, 1
    PRINT CHR$(186);
    LOCATE row, WIDTH
    PRINT CHR$(186);
NEXT row

' Bottom border
LOCATE HEIGHT, 1
PRINT CHR$(200);
FOR i = 2 TO WIDTH - 1
    PRINT CHR$(205);
NEXT i
PRINT CHR$(188);

RETURN

' === DRAW SNAKE SUBROUTINE ===
700 COLOR 10, 1
FOR i = 0 TO snakeLen - 1
    LOCATE snakeY(i), snakeX(i)
    PRINT CHR$(219);
NEXT i
RETURN

' === DRAW FOOD SUBROUTINE ===
800 COLOR 12, 1
LOCATE foodY, foodX
PRINT "*";
RETURN

' === DRAW SCORE SUBROUTINE ===
900 COLOR 14, 1
LOCATE HEIGHT + 1, 2
PRINT "Score: "; score; "   Length: "; snakeLen; "    ";
RETURN

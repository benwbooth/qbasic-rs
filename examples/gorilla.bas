' GORILLA.BAS - A QBasic Classic
' Two gorillas throwing exploding bananas at each other

' Initialize arrays
DIM buildingX(15)
DIM buildingW(15)
DIM buildingH(15)

' Constants
scrWidth = 640
scrHeight = 480
gravity = 0.5
numBuildings = 9
score1 = 0
score2 = 0

' Main game loop
GOSUB StartNewRound

gameOver = 0
currentPlayer = 1

WHILE gameOver = 0
    GOSUB DrawGame

    ' Get player input
    IF currentPlayer = 1 THEN
        LOCATE 1, 1
        COLOR 15
        PRINT "Player 1 - Score:"; score1
        LOCATE 2, 1
        INPUT "Angle: ", angle
        LOCATE 3, 1
        INPUT "Power: ", velocity
        startX = gorilla1X + 20
        startY = gorilla1Y - 10
        direction = 1
    ELSE
        LOCATE 1, 50
        COLOR 15
        PRINT "Player 2 - Score:"; score2
        LOCATE 2, 50
        INPUT "Angle: ", angle
        LOCATE 3, 50
        INPUT "Power: ", velocity
        startX = gorilla2X + 20
        startY = gorilla2Y - 10
        direction = -1
    END IF

    ' Clamp values
    IF angle < 0 THEN angle = 0
    IF angle > 90 THEN angle = 90
    IF velocity < 1 THEN velocity = 1
    IF velocity > 100 THEN velocity = 100

    ' Clear input area by redrawing
    GOSUB DrawGame

    ' Launch banana
    GOSUB ThrowBanana

    ' Check result
    IF hitTarget = 1 THEN
        IF currentPlayer = 1 THEN
            score1 = score1 + 1
            GOSUB DrawExplosion
            IF score1 >= 3 THEN
                gameOver = 1
                winner = 1
            ELSE
                GOSUB StartNewRound
            END IF
        ELSE
            score2 = score2 + 1
            GOSUB DrawExplosion
            IF score2 >= 3 THEN
                gameOver = 1
                winner = 2
            ELSE
                GOSUB StartNewRound
            END IF
        END IF
    ELSE
        ' Switch players
        IF currentPlayer = 1 THEN
            currentPlayer = 2
        ELSE
            currentPlayer = 1
        END IF
    END IF
WEND

' Game over screen
CLS
COLOR 14
LOCATE 10, 35
PRINT "GAME OVER!"
LOCATE 12, 30
PRINT "Player"; winner; "wins!"
LOCATE 14, 30
PRINT "Final Score:"
LOCATE 16, 30
PRINT "Player 1:"; score1
LOCATE 17, 30
PRINT "Player 2:"; score2
END

' ================================
' Subroutines
' ================================

StartNewRound:
    ' Generate random wind
    wind = (RND * 0.4) - 0.2

    ' Generate random buildings
    currentX = 0
    FOR i = 0 TO numBuildings - 1
        buildingW(i) = 50 + INT(RND * 30)
        buildingH(i) = 100 + INT(RND * 200)
        buildingX(i) = currentX
        currentX = currentX + buildingW(i)
    NEXT i

    ' Place gorillas on buildings
    gorilla1X = buildingX(1) + buildingW(1) / 2 - 20
    gorilla1Y = scrHeight - buildingH(1) - 40

    gorilla2X = buildingX(numBuildings - 2) + buildingW(numBuildings - 2) / 2 - 20
    gorilla2Y = scrHeight - buildingH(numBuildings - 2) - 40
RETURN

DrawGame:
    ' Clear with blue sky
    COLOR 1
    LINE (0, 0)-(scrWidth, scrHeight), , BF

    ' Draw sun
    COLOR 14
    CIRCLE (scrWidth - 80, 60), 40
    PAINT (scrWidth - 80, 60), 14

    ' Draw buildings
    FOR i = 0 TO numBuildings - 1
        bx = buildingX(i)
        bh = buildingH(i)
        bw = buildingW(i)

        ' Building body (dark gray)
        COLOR 8
        LINE (bx, scrHeight - bh)-(bx + bw, scrHeight), , BF

        ' Windows (yellow)
        COLOR 14
        FOR wy = scrHeight - bh + 15 TO scrHeight - 15 STEP 25
            FOR wx = bx + 8 TO bx + bw - 15 STEP 18
                LINE (wx, wy)-(wx + 10, wy + 15), , BF
            NEXT wx
        NEXT wy
    NEXT i

    ' Draw gorillas
    GOSUB DrawGorilla1
    GOSUB DrawGorilla2

    ' Draw wind indicator
    COLOR 15
    LOCATE 1, 35
    IF wind > 0.05 THEN
        PRINT "Wind: -->"
    ELSEIF wind < -0.05 THEN
        PRINT "Wind: <--"
    ELSE
        PRINT "Wind: ---"
    END IF
RETURN

DrawGorilla1:
    COLOR 6
    ' Body
    LINE (gorilla1X, gorilla1Y)-(gorilla1X + 40, gorilla1Y + 40), , BF
    ' Head
    CIRCLE (gorilla1X + 20, gorilla1Y - 15), 18
    PAINT (gorilla1X + 20, gorilla1Y - 15), 6
    ' Arms
    LINE (gorilla1X - 8, gorilla1Y + 5)-(gorilla1X, gorilla1Y + 35), , BF
    LINE (gorilla1X + 40, gorilla1Y + 5)-(gorilla1X + 48, gorilla1Y + 35), , BF
    ' Eyes
    COLOR 15
    CIRCLE (gorilla1X + 14, gorilla1Y - 18), 3
    CIRCLE (gorilla1X + 26, gorilla1Y - 18), 3
    PAINT (gorilla1X + 14, gorilla1Y - 18), 15
    PAINT (gorilla1X + 26, gorilla1Y - 18), 15
RETURN

DrawGorilla2:
    COLOR 6
    ' Body
    LINE (gorilla2X, gorilla2Y)-(gorilla2X + 40, gorilla2Y + 40), , BF
    ' Head
    CIRCLE (gorilla2X + 20, gorilla2Y - 15), 18
    PAINT (gorilla2X + 20, gorilla2Y - 15), 6
    ' Arms
    LINE (gorilla2X - 8, gorilla2Y + 5)-(gorilla2X, gorilla2Y + 35), , BF
    LINE (gorilla2X + 40, gorilla2Y + 5)-(gorilla2X + 48, gorilla2Y + 35), , BF
    ' Eyes
    COLOR 15
    CIRCLE (gorilla2X + 14, gorilla2Y - 18), 3
    CIRCLE (gorilla2X + 26, gorilla2Y - 18), 3
    PAINT (gorilla2X + 14, gorilla2Y - 18), 15
    PAINT (gorilla2X + 26, gorilla2Y - 18), 15
RETURN

ThrowBanana:
    hitTarget = 0
    hitGround = 0

    ' Convert angle to radians
    pi = 3.14159
    radAngle = angle * pi / 180

    ' Initial velocity components
    vx = velocity * COS(radAngle) * direction * 0.3
    vy = -velocity * SIN(radAngle) * 0.3

    ' Banana position
    bx = startX
    by = startY

    ' Animation loop
    WHILE hitGround = 0 AND hitTarget = 0
        ' Update velocity (gravity and wind)
        vy = vy + gravity
        vx = vx + wind

        ' Update position
        bx = bx + vx
        by = by + vy

        ' Bounds check
        IF bx < 0 OR bx > scrWidth OR by > scrHeight THEN
            hitGround = 1
        ELSE
            ' Draw banana
            COLOR 14
            CIRCLE (bx, by), 5
            PAINT (bx, by), 14

            ' Check collision with opponent gorilla
            IF currentPlayer = 1 THEN
                IF bx >= gorilla2X - 10 AND bx <= gorilla2X + 50 THEN
                    IF by >= gorilla2Y - 35 AND by <= gorilla2Y + 45 THEN
                        hitTarget = 1
                        explosionX = bx
                        explosionY = by
                    END IF
                END IF
            ELSE
                IF bx >= gorilla1X - 10 AND bx <= gorilla1X + 50 THEN
                    IF by >= gorilla1Y - 35 AND by <= gorilla1Y + 45 THEN
                        hitTarget = 1
                        explosionX = bx
                        explosionY = by
                    END IF
                END IF
            END IF

            ' Check collision with buildings
            IF hitTarget = 0 THEN
                FOR i = 0 TO numBuildings - 1
                    bldgTop = scrHeight - buildingH(i)
                    IF bx >= buildingX(i) AND bx <= buildingX(i) + buildingW(i) THEN
                        IF by >= bldgTop AND by <= scrHeight THEN
                            hitGround = 1
                            explosionX = bx
                            explosionY = by
                        END IF
                    END IF
                NEXT i
            END IF
        END IF

        ' Small delay for animation
        FOR delay = 1 TO 100
        NEXT delay
    WEND
RETURN

DrawExplosion:
    ' Draw explosion animation
    FOR radius = 10 TO 80 STEP 10
        COLOR 12
        CIRCLE (explosionX, explosionY), radius
        COLOR 14
        CIRCLE (explosionX, explosionY), radius - 5
        PAINT (explosionX, explosionY), 14

        FOR delay = 1 TO 500
        NEXT delay
    NEXT radius

    ' Final flash
    COLOR 15
    CIRCLE (explosionX, explosionY), 100
    PAINT (explosionX, explosionY), 15

    FOR delay = 1 TO 1000
    NEXT delay
RETURN

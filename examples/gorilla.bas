' GORILLA.BAS - A QBasic Classic
' Two gorillas throwing exploding bananas at each other

SCREEN 12

' Initialize arrays
DIM buildingX(15)
DIM buildingW(15)
DIM buildingH(15)

' Get screen dimensions (updates on resize)
scrWidth = SCREENWIDTH
scrHeight = SCREENHEIGHT

' Game settings
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

    ' Get player input (use scaled gorilla size for positions)
    gs = scrHeight / 12
    IF currentPlayer = 1 THEN
        LOCATE 1, 1
        COLOR 15
        PRINT "Player 1 - Score:"; score1
        LOCATE 2, 1
        INPUT "Angle: ", angle
        LOCATE 3, 1
        INPUT "Power: ", velocity
        startX = gorilla1X + gs / 2
        startY = gorilla1Y - gs / 4
        direction = 1
    ELSE
        LOCATE 1, 50
        COLOR 15
        PRINT "Player 2 - Score:"; score2
        LOCATE 2, 50
        INPUT "Angle: ", angle
        LOCATE 3, 50
        INPUT "Power: ", velocity
        startX = gorilla2X + gs / 2
        startY = gorilla2Y - gs / 4
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
    ' Refresh screen dimensions
    scrWidth = SCREENWIDTH
    scrHeight = SCREENHEIGHT

    ' Generate random wind
    wind = (RND * 0.4) - 0.2

    ' Generate random buildings scaled to screen width
    buildingBaseW = scrWidth / numBuildings
    currentX = 0
    FOR i = 0 TO numBuildings - 1
        buildingW(i) = buildingBaseW * 0.7 + INT(RND * buildingBaseW * 0.3)
        buildingH(i) = scrHeight * 0.2 + INT(RND * scrHeight * 0.4)
        buildingX(i) = currentX
        currentX = currentX + buildingW(i)
    NEXT i

    ' Place gorillas on buildings (use scaled gorilla size)
    gs = scrHeight / 12
    gorilla1X = buildingX(1) + buildingW(1) / 2 - gs / 2
    gorilla1Y = scrHeight - buildingH(1) - gs

    gorilla2X = buildingX(numBuildings - 2) + buildingW(numBuildings - 2) / 2 - gs / 2
    gorilla2Y = scrHeight - buildingH(numBuildings - 2) - gs
RETURN

DrawGame:
    ' Refresh screen dimensions for resize handling
    scrWidth = SCREENWIDTH
    scrHeight = SCREENHEIGHT

    ' Clear with blue sky
    COLOR 1
    LINE (0, 0)-(scrWidth, scrHeight), , BF

    ' Draw sun (scaled position)
    sunX = scrWidth - scrWidth / 8
    sunY = scrHeight / 8
    sunR = scrHeight / 12
    COLOR 14
    CIRCLE (sunX, sunY), sunR
    PAINT (sunX, sunY), 14

    ' Draw buildings
    ' Window sizes scaled to screen
    winW = scrHeight / 48
    winH = scrHeight / 32
    winSpaceX = scrHeight / 27
    winSpaceY = scrHeight / 19
    winMargin = scrHeight / 53

    FOR i = 0 TO numBuildings - 1
        bx = buildingX(i)
        bh = buildingH(i)
        bw = buildingW(i)

        ' Building body (dark gray)
        COLOR 8
        LINE (bx, scrHeight - bh)-(bx + bw, scrHeight), , BF

        ' Windows (yellow)
        COLOR 14
        FOR wy = scrHeight - bh + winMargin TO scrHeight - winMargin STEP winSpaceY
            FOR wx = bx + winMargin TO bx + bw - winMargin - winW STEP winSpaceX
                LINE (wx, wy)-(wx + winW, wy + winH), , BF
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
    ' Draw gorilla at gorilla1X, gorilla1Y (matching original GORILLA.BAS style)
    gx = gorilla1X
    gy = gorilla1Y
    gs = scrHeight / 12
    pi = 3.14159
    COLOR 6

    ' Head (two overlapping boxes like original)
    LINE (gx, gy)-(gx + gs*0.7, gy + gs*0.4), , BF
    LINE (gx - gs*0.1, gy + gs*0.15)-(gx + gs*0.8, gy + gs*0.3), , BF

    ' Eyes/brow (dark line across face)
    COLOR 0
    LINE (gx + gs*0.05, gy + gs*0.15)-(gx + gs*0.55, gy + gs*0.15)

    ' Neck
    COLOR 6
    LINE (gx + gs*0.05, gy + gs*0.45)-(gx + gs*0.55, gy + gs*0.45)

    ' Upper body
    LINE (gx - gs*0.35, gy + gs*0.5)-(gx + gs*1.0, gy + gs*0.85), , BF

    ' Lower body
    LINE (gx - gs*0.15, gy + gs*0.9)-(gx + gs*0.8, gy + gs*1.25), , BF

    ' Left arm (down position)
    CIRCLE (gx - gs*0.1, gy + gs*0.85), gs*0.5, 6, 3*pi/4, 5*pi/4

    ' Right arm (down position)
    CIRCLE (gx + gs*0.75, gy + gs*0.85), gs*0.5, 6, 7*pi/4, pi/4

    ' Legs
    CIRCLE (gx + gs*0.2, gy + gs*1.5), gs*0.5, 6, 3*pi/4, 9*pi/8
    CIRCLE (gx + gs*0.45, gy + gs*1.5), gs*0.5, 6, 15*pi/8, pi/4
RETURN

DrawGorilla2:
    ' Draw gorilla at gorilla2X, gorilla2Y (matching original GORILLA.BAS style)
    gx = gorilla2X
    gy = gorilla2Y
    gs = scrHeight / 12
    pi = 3.14159
    COLOR 6

    ' Head (two overlapping boxes like original)
    LINE (gx, gy)-(gx + gs*0.7, gy + gs*0.4), , BF
    LINE (gx - gs*0.1, gy + gs*0.15)-(gx + gs*0.8, gy + gs*0.3), , BF

    ' Eyes/brow (dark line across face)
    COLOR 0
    LINE (gx + gs*0.05, gy + gs*0.15)-(gx + gs*0.55, gy + gs*0.15)

    ' Neck
    COLOR 6
    LINE (gx + gs*0.05, gy + gs*0.45)-(gx + gs*0.55, gy + gs*0.45)

    ' Upper body
    LINE (gx - gs*0.35, gy + gs*0.5)-(gx + gs*1.0, gy + gs*0.85), , BF

    ' Lower body
    LINE (gx - gs*0.15, gy + gs*0.9)-(gx + gs*0.8, gy + gs*1.25), , BF

    ' Left arm (down position)
    CIRCLE (gx - gs*0.1, gy + gs*0.85), gs*0.5, 6, 3*pi/4, 5*pi/4

    ' Right arm (down position)
    CIRCLE (gx + gs*0.75, gy + gs*0.85), gs*0.5, 6, 7*pi/4, pi/4

    ' Legs
    CIRCLE (gx + gs*0.2, gy + gs*1.5), gs*0.5, 6, 3*pi/4, 9*pi/8
    CIRCLE (gx + gs*0.45, gy + gs*1.5), gs*0.5, 6, 15*pi/8, pi/4
RETURN

ThrowBanana:
    hitTarget = 0
    hitGround = 0

    ' Scale factors
    gs = scrHeight / 12
    bananaR = gs * 0.125

    ' Convert angle to radians
    pi = 3.14159
    radAngle = angle * pi / 180

    ' Initial velocity components (scale with screen)
    velScale = scrHeight / 480
    vx = velocity * COS(radAngle) * direction * 0.3 * velScale
    vy = -velocity * SIN(radAngle) * 0.3 * velScale
    gravityScaled = gravity * velScale

    ' Banana position
    bx = startX
    by = startY

    ' Animation loop
    WHILE hitGround = 0 AND hitTarget = 0
        ' Update velocity (gravity and wind)
        vy = vy + gravityScaled
        vx = vx + wind * velScale

        ' Update position
        bx = bx + vx
        by = by + vy

        ' Bounds check
        IF bx < 0 OR bx > scrWidth OR by > scrHeight THEN
            hitGround = 1
        ELSE
            ' Draw banana
            COLOR 14
            CIRCLE (bx, by), bananaR
            PAINT (bx, by), 14

            ' Check collision with opponent gorilla (scaled hitbox)
            IF currentPlayer = 1 THEN
                IF bx >= gorilla2X - gs*0.25 AND bx <= gorilla2X + gs*1.25 THEN
                    IF by >= gorilla2Y - gs*0.875 AND by <= gorilla2Y + gs*1.125 THEN
                        hitTarget = 1
                        explosionX = bx
                        explosionY = by
                    END IF
                END IF
            ELSE
                IF bx >= gorilla1X - gs*0.25 AND bx <= gorilla1X + gs*1.25 THEN
                    IF by >= gorilla1Y - gs*0.875 AND by <= gorilla1Y + gs*1.125 THEN
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
    ' Draw explosion animation (scaled to screen)
    expScale = scrHeight / 480
    expMax = 80 * expScale
    expStep = 10 * expScale
    FOR radius = expStep TO expMax STEP expStep
        COLOR 12
        CIRCLE (explosionX, explosionY), radius
        COLOR 14
        CIRCLE (explosionX, explosionY), radius - expStep/2
        PAINT (explosionX, explosionY), 14

        FOR delay = 1 TO 500
        NEXT delay
    NEXT radius

    ' Final flash
    COLOR 15
    CIRCLE (explosionX, explosionY), expMax * 1.25
    PAINT (explosionX, explosionY), 15

    FOR delay = 1 TO 1000
    NEXT delay
RETURN

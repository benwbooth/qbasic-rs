' GORILLA.BAS - A QBasic Classic
' Two gorillas throwing exploding bananas at each other

SCREEN 12
RANDOMIZE

' Initialize arrays
DIM buildingX(15)
DIM buildingW(15)
DIM buildingH(15)
DIM buildingC(15)

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
        PRINT "Player 1 - Score:"; score1;
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
        PRINT "Player 2 - Score:"; score2;
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

    ' Generate random buildings that fill entire screen width
    buildingBaseW = scrWidth / numBuildings
    currentX = 0
    FOR i = 0 TO numBuildings - 1
        buildingX(i) = currentX
        ' Last building extends to edge of screen
        IF i = numBuildings - 1 THEN
            buildingW(i) = scrWidth - currentX
        ELSE
            ' Random width variation but ensure we fill the space
            buildingW(i) = buildingBaseW * 0.8 + INT(RND * buildingBaseW * 0.4)
        END IF
        buildingH(i) = scrHeight * 0.25 + INT(RND * scrHeight * 0.35)
        ' Assign random color (4=red, 5=magenta, 7=white, 3=cyan, 8=gray)
        colorPick = INT(RND * 5)
        IF colorPick = 0 THEN
            buildingC(i) = 4
        ELSEIF colorPick = 1 THEN
            buildingC(i) = 5
        ELSEIF colorPick = 2 THEN
            buildingC(i) = 7
        ELSEIF colorPick = 3 THEN
            buildingC(i) = 3
        ELSE
            buildingC(i) = 8
        END IF
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

    ' Draw sun (centered at top, smaller, with rays and smiley face)
    sunX = scrWidth / 2
    sunY = scrHeight / 10
    sunR = scrHeight / 20
    rayLen = sunR * 0.6

    ' Draw rays first (behind sun)
    COLOR 14
    pi = 3.14159
    FOR rayAngle = 0 TO 7
        angle = rayAngle * pi / 4
        x1 = sunX + (sunR + 2) * COS(angle)
        y1 = sunY - (sunR + 2) * SIN(angle)
        x2 = sunX + (sunR + rayLen) * COS(angle)
        y2 = sunY - (sunR + rayLen) * SIN(angle)
        ' Draw thick ray using bezier
        BEZIER (x1, y1)-(x2, y2 - rayLen*0.2)-(x2, y2), 14, 3
    NEXT rayAngle

    ' Draw sun circle
    CIRCLE (sunX, sunY), sunR
    PAINT (sunX, sunY), 14

    ' Draw smiley face
    COLOR 0
    ' Eyes
    eyeOffset = sunR * 0.3
    eyeY = sunY - sunR * 0.15
    CIRCLE (sunX - eyeOffset, eyeY), sunR * 0.08
    PAINT (sunX - eyeOffset, eyeY), 0
    CIRCLE (sunX + eyeOffset, eyeY), sunR * 0.08
    PAINT (sunX + eyeOffset, eyeY), 0
    ' Smile (arc)
    smileY = sunY + sunR * 0.1
    smileR = sunR * 0.45
    CIRCLE (sunX, smileY), smileR, 0, pi, 2*pi

    ' Draw buildings
    ' Window sizes scaled to screen
    winW = scrHeight / 48
    winH = scrHeight / 32
    winSpaceX = scrHeight / 20
    winSpaceY = scrHeight / 16
    winMargin = scrHeight / 40

    FOR i = 0 TO numBuildings - 1
        bx = buildingX(i)
        bh = buildingH(i)
        bw = buildingW(i)
        bc = buildingC(i)
        LOCATE 5 + i, 1
        PRINT "Draw"; i; "bc="; bc;

        ' Building body (use building's color)
        COLOR bc
        LINE (bx, scrHeight - bh)-(bx + bw, scrHeight), , BF

        ' Windows (randomly yellow or black/dark)
        FOR wy = scrHeight - bh + winMargin TO scrHeight - winMargin - winH STEP winSpaceY
            FOR wx = bx + winMargin TO bx + bw - winMargin - winW STEP winSpaceX
                ' 70% chance of lit window (yellow), 30% dark
                IF RND > 0.3 THEN
                    COLOR 14  ' Yellow (lit)
                ELSE
                    COLOR 0   ' Black (dark)
                END IF
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
        PRINT "Wind: -->";
    ELSEIF wind < -0.05 THEN
        PRINT "Wind: <--";
    ELSE
        PRINT "Wind: ---";
    END IF
RETURN

DrawGorilla1:
    ' Draw gorilla using ellipses and bezier curves (SVG-inspired design)
    ' gs = gorilla size unit (based on screen height)
    gs = scrHeight / 12
    ' Center the gorilla on its position
    cx = gorilla1X + gs * 0.35
    cy = gorilla1Y + gs * 0.2

    COLOR 6

    ' Head (ellipse)
    CIRCLE (cx, cy), gs*0.22, 6, 0, 6.28, 0.82
    PAINT (cx, cy), 6

    ' Brow ridge (wide ellipse)
    CIRCLE (cx, cy + gs*0.04), gs*0.26, 6, 0, 6.28, 0.31
    PAINT (cx, cy + gs*0.04), 6

    ' Muzzle/face
    CIRCLE (cx, cy + gs*0.1), gs*0.14, 6, 0, 6.28, 0.71
    PAINT (cx, cy + gs*0.1), 6

    ' Eyes (black ellipses)
    COLOR 0
    CIRCLE (cx - gs*0.1, cy + gs*0.02), gs*0.04, 0, 0, 6.28, 0.75
    PAINT (cx - gs*0.1, cy + gs*0.02), 0
    CIRCLE (cx + gs*0.1, cy + gs*0.02), gs*0.04, 0, 0, 6.28, 0.75
    PAINT (cx + gs*0.1, cy + gs*0.02), 0

    ' Nostrils
    CIRCLE (cx - gs*0.04, cy + gs*0.12), gs*0.02, 0
    PAINT (cx - gs*0.04, cy + gs*0.12), 0
    CIRCLE (cx + gs*0.04, cy + gs*0.12), gs*0.02, 0
    PAINT (cx + gs*0.04, cy + gs*0.12), 0

    ' Neck
    COLOR 6
    LINE (cx - gs*0.12, cy + gs*0.16)-(cx + gs*0.12, cy + gs*0.24), , BF

    ' Torso (wide ellipse)
    CIRCLE (cx, cy + gs*0.36), gs*0.34, 6, 0, 6.28, 0.47
    PAINT (cx, cy + gs*0.36), 6

    ' Belly/hips (ellipse)
    CIRCLE (cx, cy + gs*0.58), gs*0.24, 6, 0, 6.28, 0.58
    PAINT (cx, cy + gs*0.58), 6

    ' Left arm (bezier curve with thickness)
    armThick = gs * 0.12
    BEZIER (cx - gs*0.30, cy + gs*0.30)-(cx - gs*0.48, cy + gs*0.48)-(cx - gs*0.40, cy + gs*0.70), 6, armThick

    ' Right arm
    BEZIER (cx + gs*0.30, cy + gs*0.30)-(cx + gs*0.48, cy + gs*0.48)-(cx + gs*0.40, cy + gs*0.70), 6, armThick

    ' Left leg (bezier curve with thickness)
    legThick = gs * 0.12
    BEZIER (cx - gs*0.16, cy + gs*0.68)-(cx - gs*0.28, cy + gs*0.83)-(cx - gs*0.24, cy + gs*0.96), 6, legThick

    ' Right leg
    BEZIER (cx + gs*0.16, cy + gs*0.68)-(cx + gs*0.28, cy + gs*0.83)-(cx + gs*0.24, cy + gs*0.96), 6, legThick
RETURN

DrawGorilla2:
    ' Draw gorilla using ellipses and bezier curves (SVG-inspired design)
    ' gs = gorilla size unit (based on screen height)
    gs = scrHeight / 12
    ' Center the gorilla on its position
    cx = gorilla2X + gs * 0.35
    cy = gorilla2Y + gs * 0.2

    COLOR 6

    ' Head (ellipse)
    CIRCLE (cx, cy), gs*0.22, 6, 0, 6.28, 0.82
    PAINT (cx, cy), 6

    ' Brow ridge (wide ellipse)
    CIRCLE (cx, cy + gs*0.04), gs*0.26, 6, 0, 6.28, 0.31
    PAINT (cx, cy + gs*0.04), 6

    ' Muzzle/face
    CIRCLE (cx, cy + gs*0.1), gs*0.14, 6, 0, 6.28, 0.71
    PAINT (cx, cy + gs*0.1), 6

    ' Eyes (black ellipses)
    COLOR 0
    CIRCLE (cx - gs*0.1, cy + gs*0.02), gs*0.04, 0, 0, 6.28, 0.75
    PAINT (cx - gs*0.1, cy + gs*0.02), 0
    CIRCLE (cx + gs*0.1, cy + gs*0.02), gs*0.04, 0, 0, 6.28, 0.75
    PAINT (cx + gs*0.1, cy + gs*0.02), 0

    ' Nostrils
    CIRCLE (cx - gs*0.04, cy + gs*0.12), gs*0.02, 0
    PAINT (cx - gs*0.04, cy + gs*0.12), 0
    CIRCLE (cx + gs*0.04, cy + gs*0.12), gs*0.02, 0
    PAINT (cx + gs*0.04, cy + gs*0.12), 0

    ' Neck
    COLOR 6
    LINE (cx - gs*0.12, cy + gs*0.16)-(cx + gs*0.12, cy + gs*0.24), , BF

    ' Torso (wide ellipse)
    CIRCLE (cx, cy + gs*0.36), gs*0.34, 6, 0, 6.28, 0.47
    PAINT (cx, cy + gs*0.36), 6

    ' Belly/hips (ellipse)
    CIRCLE (cx, cy + gs*0.58), gs*0.24, 6, 0, 6.28, 0.58
    PAINT (cx, cy + gs*0.58), 6

    ' Left arm (bezier curve with thickness)
    armThick = gs * 0.12
    BEZIER (cx - gs*0.30, cy + gs*0.30)-(cx - gs*0.48, cy + gs*0.48)-(cx - gs*0.40, cy + gs*0.70), 6, armThick

    ' Right arm
    BEZIER (cx + gs*0.30, cy + gs*0.30)-(cx + gs*0.48, cy + gs*0.48)-(cx + gs*0.40, cy + gs*0.70), 6, armThick

    ' Left leg (bezier curve with thickness)
    legThick = gs * 0.12
    BEZIER (cx - gs*0.16, cy + gs*0.68)-(cx - gs*0.28, cy + gs*0.83)-(cx - gs*0.24, cy + gs*0.96), 6, legThick

    ' Right leg
    BEZIER (cx + gs*0.16, cy + gs*0.68)-(cx + gs*0.28, cy + gs*0.83)-(cx + gs*0.24, cy + gs*0.96), 6, legThick
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

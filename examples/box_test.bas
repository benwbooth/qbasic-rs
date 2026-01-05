' Simple test for LINE with BF (box fill)
SCREEN 12

' Draw a blue background
COLOR 1
LINE (0, 0)-(639, 479), , BF

' Draw a red box in the middle
COLOR 4
LINE (200, 150)-(400, 300), , BF

' Draw a yellow box
COLOR 14
LINE (50, 50)-(150, 100), , BF

' Draw a green box at bottom
COLOR 2
LINE (100, 400)-(300, 450), , BF

END

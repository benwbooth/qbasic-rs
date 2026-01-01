# LOCATE Statement

Positions the cursor on the screen.

## Syntax
```
LOCATE row, column
```

## Example
```
CLS
LOCATE 12, 35
PRINT "Center!"
LOCATE 25, 1
PRINT "Bottom left"
```

## Notes
- Row ranges from 1 to 25 (typically)
- Column ranges from 1 to 80 (typically)
- Cursor position affects next PRINT
- Use with COLOR for formatted output

## See Also
- [PRINT](print)
- [COLOR](color)
- [CLS](cls)

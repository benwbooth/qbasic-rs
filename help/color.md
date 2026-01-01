# COLOR Statement

Sets foreground and background text colors.

## Syntax
```
COLOR foreground [, background]
```

## Colors
| Code | Color | Code | Color |
|------|-------|------|-------|
| 0 | Black | 8 | Dark Gray |
| 1 | Blue | 9 | Light Blue |
| 2 | Green | 10 | Light Green |
| 3 | Cyan | 11 | Light Cyan |
| 4 | Red | 12 | Light Red |
| 5 | Magenta | 13 | Light Magenta |
| 6 | Brown | 14 | Yellow |
| 7 | Light Gray | 15 | White |

## Example
```
COLOR 14, 1       ' Yellow on blue
PRINT "Warning!"
COLOR 7, 0        ' Reset to default
```

## See Also
- [CLS](cls)
- [LOCATE](locate)
- [PRINT](print)

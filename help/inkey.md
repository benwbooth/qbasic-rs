# INKEY$ Function

Reads a key from the keyboard without waiting.

## Syntax
```
INKEY$
```

## Example
```
DO
  k$ = INKEY$
  IF k$ <> "" THEN
    PRINT "You pressed: "; k$
  END IF
LOOP UNTIL k$ = CHR$(27)  ' ESC
```

## Notes
- Returns empty string if no key pressed
- Does not echo to screen
- Use in game loops for real-time input
- Special keys return two-character codes

## See Also
- [INPUT](input)
- [ASC](asc)
- [CHR$](chr)

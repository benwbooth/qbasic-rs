# MID$ Function

Returns a substring from the middle of a string.

## Syntax
```
MID$(string$, start [, length])
```

## Example
```
PRINT MID$("Hello", 2, 3)  ' Prints "ell"
PRINT MID$("Hello", 2)     ' Prints "ello"
s$ = "ABCDEF"
PRINT MID$(s$, 3, 2)       ' Prints "CD"
```

## Notes
- Start position is 1-based
- If length omitted, returns rest of string
- Returns empty string if start > length

## See Also
- [LEFT$](left)
- [RIGHT$](right)
- [LEN](len)

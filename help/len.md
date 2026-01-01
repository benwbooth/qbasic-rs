# LEN Function

Returns the length of a string.

## Syntax
```
LEN(string$)
```

## Example
```
PRINT LEN("Hello")  ' Prints 5
PRINT LEN("")       ' Prints 0
s$ = "QBasic"
FOR i = 1 TO LEN(s$)
  PRINT MID$(s$, i, 1)
NEXT
```

## See Also
- [LEFT$](left)
- [RIGHT$](right)
- [MID$](mid)

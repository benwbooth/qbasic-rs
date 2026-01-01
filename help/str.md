# STR$ Function

Converts a number to a string.

## Syntax
```
STR$(number)
```

## Example
```
PRINT STR$(42)        ' Prints " 42"
n = 3.14
s$ = STR$(n)
PRINT "Value:" + s$
```

## Notes
- Positive numbers have leading space
- Use LTRIM$ to remove leading space
- Opposite of VAL function

## See Also
- [VAL](val)
- [Functions](functions)

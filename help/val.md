# VAL Function

Converts a string to a number.

## Syntax
```
VAL(string$)
```

## Example
```
PRINT VAL("42")       ' Prints 42
PRINT VAL("3.14")     ' Prints 3.14
PRINT VAL("12abc")    ' Prints 12
PRINT VAL("abc")      ' Prints 0
```

## Notes
- Stops at first non-numeric character
- Returns 0 if string doesn't start with number
- Opposite of STR$ function

## See Also
- [STR$](str)
- [Functions](functions)

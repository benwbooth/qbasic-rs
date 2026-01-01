# INT Function

Returns the integer part of a number (truncates toward negative infinity).

## Syntax
```
INT(number)
```

## Example
```
PRINT INT(3.7)   ' Prints 3
PRINT INT(-3.7)  ' Prints -4
PRINT INT(5)     ' Prints 5
```

## Notes
- Rounds toward negative infinity
- INT(-3.2) = -4, not -3
- Use for floor operation

## See Also
- [ABS](abs)
- [Functions](functions)

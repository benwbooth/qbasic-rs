# Data Types

## Numeric Types
- **Integer** - Whole numbers (-32768 to 32767)
- **Long** - Large integers
- **Single** - Decimal numbers (default)
- **Double** - High precision decimals

## Type Suffixes
| Suffix | Type | Example |
|--------|------|---------|
| % | Integer | count% |
| & | Long | bignum& |
| ! | Single | price! |
| # | Double | precise# |
| $ | String | name$ |

## Strings
Strings hold text and end with $:
```
name$ = "Hello"
PRINT LEN(name$)   ' Prints 5
```

## Arrays
Use DIM to create arrays:
```
DIM scores(10)     ' 11 elements (0-10)
DIM names$(5)      ' String array
DIM grid(10, 10)   ' 2D array
```

## Type Conversion
- VAL(s$) - String to number
- STR$(n) - Number to string
- INT(n) - Truncate to integer
- CHR$(n) - ASCII to character
- ASC(c$) - Character to ASCII

## See Also
- [DIM](dim)
- [Functions](functions)

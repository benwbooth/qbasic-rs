# DIM Statement

Declares arrays with specified dimensions.

## Syntax
```
DIM arrayname(subscript [, subscript...])
```

## Examples
```
DIM scores(10)         ' Array with indices 0-10
DIM names$(20)         ' String array
DIM grid(10, 10)       ' Two-dimensional array
DIM cube(5, 5, 5)      ' Three-dimensional array
```

## Notes
- Arrays are zero-indexed by default
- Maximum subscript is the size (0 to N)
- Use $ suffix for string arrays
- Arrays must be dimensioned before use

## See Also
- [Data Types](types)
- [LET](let)

# WHILE...WEND Statement

Repeats a block while a condition is true.

## Syntax
```
WHILE condition
  ' statements
WEND
```

## Example
```
count = 1
WHILE count <= 10
  PRINT count
  count = count + 1
WEND
```

## Notes
- Condition is checked before each iteration
- Loop may execute zero times
- Use EXIT WHILE to leave early (if supported)
- Avoid infinite loops (ensure condition changes)

## See Also
- [FOR...NEXT](for)
- [IF...THEN](if)
- [GOTO](goto)

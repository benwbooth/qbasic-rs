# GOSUB...RETURN Statement

Calls a subroutine and returns when done.

## Syntax
```
GOSUB label
...
label:
  ' subroutine code
RETURN
```

## Example
```
PRINT "Start"
GOSUB PrintHello
PRINT "End"
END

PrintHello:
  PRINT "Hello, World!"
RETURN
```

## Notes
- RETURN jumps back after GOSUB
- Subroutines can be nested
- Use END before subroutines to prevent fall-through
- Labels end with a colon (:)

## See Also
- [GOTO](goto)
- [END](end)
- [SUB](statements)

# GOTO Statement

Jumps to a labeled line in the program.

## Syntax
```
GOTO label
```

## Example
```
start:
PRINT "Hello"
INPUT "Again? (y/n) "; a$
IF a$ = "y" THEN GOTO start
PRINT "Goodbye"
```

## Notes
- Labels are names followed by colon (:)
- Avoid excessive GOTO for readable code
- Use loops (FOR, WHILE) when possible
- Cannot jump into or out of SUB/FUNCTION

## See Also
- [GOSUB](gosub)
- [IF...THEN](if)
- [FOR...NEXT](for)

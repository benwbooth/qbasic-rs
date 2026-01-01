# TIMER Function

Returns seconds elapsed since midnight.

## Syntax
```
TIMER
```

## Example
```
start = TIMER
' ... do something ...
elapsed = TIMER - start
PRINT "Took"; elapsed; "seconds"
```

## Game Loop Example
```
lastTime = TIMER
DO
  IF TIMER - lastTime > 0.1 THEN
    ' Update game 10 times per second
    lastTime = TIMER
  END IF
LOOP
```

## Notes
- Returns a floating-point number
- Wraps at midnight (86400 seconds)
- Use for timing and delays

## See Also
- [RND](rnd)
- [Functions](functions)

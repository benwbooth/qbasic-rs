# RND Function

Returns a random number between 0 and 1.

## Syntax
```
RND [(n)]
```

## Example
```
RANDOMIZE TIMER         ' Seed generator
PRINT RND               ' 0 to 0.999...
dice = INT(RND * 6) + 1 ' 1 to 6
```

## Random Integer Range
To get random integer from A to B:
```
result = INT(RND * (B - A + 1)) + A
```

## Notes
- Returns value >= 0 and < 1
- Use RANDOMIZE for different sequences
- Same seed gives same sequence

## See Also
- [INT](int)
- [Functions](functions)

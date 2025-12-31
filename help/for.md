# FOR...NEXT Statement

Repeats a group of statements a specified number of times.

## Syntax

```
FOR counter = start TO end [STEP increment]
    [statements]
NEXT [counter]
```

## Parameters

| Parameter | Description |
|-----------|-------------|
| counter | Numeric variable used as loop counter |
| start | Initial value of counter |
| end | Final value of counter |
| increment | Amount to add each iteration (default: 1) |

## Description

The FOR...NEXT loop executes the statements between FOR and NEXT repeatedly, incrementing the counter each time until it exceeds the end value.

- If STEP is positive, loop continues while counter <= end
- If STEP is negative, loop continues while counter >= end
- The counter variable is available after the loop ends

## Examples

```basic
' Count from 1 to 10
FOR i = 1 TO 10
    PRINT i
NEXT i

' Count by 2s
FOR i = 0 TO 20 STEP 2
    PRINT i
NEXT i

' Count backwards
FOR i = 10 TO 1 STEP -1
    PRINT i
NEXT i

' Nested loops
FOR row = 1 TO 5
    FOR col = 1 TO 10
        PRINT "*";
    NEXT col
    PRINT              ' New line
NEXT row
```

## See Also
- [WHILE...WEND](while) - Conditional loop
- [DO...LOOP](do) - Flexible loop
- [EXIT FOR](exit) - Exit loop early

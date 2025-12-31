# IF...THEN...ELSE Statement

Allows conditional execution of statements.

## Syntax

### Single-line form
```
IF condition THEN statement [ELSE statement]
```

### Block form
```
IF condition THEN
    [statements]
[ELSEIF condition THEN
    [statements]]...
[ELSE
    [statements]]
END IF
```

## Description

The IF statement executes different code based on whether a condition is true or false.

- condition is any expression that evaluates to true (non-zero) or false (zero)
- Multiple conditions can be tested with ELSEIF
- The ELSE clause is optional

## Examples

```basic
' Single-line IF
IF x > 10 THEN PRINT "Big"

' Single-line with ELSE
IF x > 0 THEN PRINT "Positive" ELSE PRINT "Not positive"

' Block IF
IF score >= 90 THEN
    PRINT "Grade: A"
ELSEIF score >= 80 THEN
    PRINT "Grade: B"
ELSEIF score >= 70 THEN
    PRINT "Grade: C"
ELSE
    PRINT "Grade: F"
END IF

' Compound conditions
IF x > 0 AND x < 100 THEN
    PRINT "x is between 0 and 100"
END IF

IF name$ = "admin" OR name$ = "root" THEN
    PRINT "Welcome, administrator!"
END IF
```

## Logical Operators

| Operator | Description |
|----------|-------------|
| AND | True if both conditions are true |
| OR | True if either condition is true |
| NOT | Inverts the condition |

## See Also
- [SELECT CASE](select) - Multi-way branching
- [Operators](operators) - Comparison operators

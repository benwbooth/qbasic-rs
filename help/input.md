# INPUT Statement

Reads input from the keyboard.

## Syntax

```
INPUT [;] ["prompt" {,|;}] variable [, variable]...
```

## Parameters

| Parameter | Description |
|-----------|-------------|
| ; | Keeps cursor on same line after input |
| prompt | Optional text displayed before input |
| variable | Variable(s) to store the input |

## Description

INPUT pauses program execution and waits for the user to type a value and press Enter.

- Use a semicolon after the prompt to suppress the `?` that normally appears
- Multiple variables can be input at once, separated by commas
- String variables (ending in $) accept text
- Numeric variables accept numbers

## Examples

```basic
' Simple input
INPUT x
PRINT "You entered:"; x

' With prompt
INPUT "Enter your name: ", name$
PRINT "Hello, "; name$

' Suppress question mark
INPUT "Age"; age
PRINT "You are"; age; "years old"

' Multiple inputs
INPUT "Enter x, y: ", x, y
PRINT "Point is at"; x; ","; y

' Keep cursor on same line
INPUT ; "Continue? ", answer$
```

## See Also
- [PRINT](print) - Display output
- [LINE INPUT](lineinput) - Input entire line
- [INKEY$](inkey) - Read single key

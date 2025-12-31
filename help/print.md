# PRINT Statement

Outputs data to the screen.

## Syntax

```
PRINT [expression] [{,|;} expression]...
```

## Description

The PRINT statement displays text and values on the screen at the current cursor position.

- Use `;` (semicolon) to print items with no space between them
- Use `,` (comma) to advance to the next print zone (14 columns)
- Use `PRINT` alone to print a blank line

## Examples

```basic
PRINT "Hello, World!"           ' Prints text
PRINT                           ' Prints blank line
PRINT "Sum is"; 2 + 2           ' Prints: Sum is 4
PRINT A, B, C                   ' Prints in columns
PRINT "Name: "; name$           ' Concatenates output
PRINT x; y; z                   ' No spaces between
PRINT "Value ="; x;             ' No newline at end
```

## Using with LOCATE

Use [LOCATE](locate) to position the cursor before printing:

```basic
LOCATE 10, 20
PRINT "Centered text"
```

## Using with COLOR

Use [COLOR](color) to set text colors before printing:

```basic
COLOR 14, 1              ' Yellow on blue
PRINT "Colorful text!"
```

## See Also
- [LOCATE](locate) - Position cursor
- [COLOR](color) - Set colors
- [INPUT](input) - Read input
- [CLS](cls) - Clear screen

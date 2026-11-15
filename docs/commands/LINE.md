# LINE (L)

Draw one or more connected line segments.

## Prompts

1. First point
2. Next point (repeat); Enter ends the command

## Input

- Absolute: `x,y`
- Relative: `@dx,dy`
- Polar: `@dist<angle_degrees`
- Mouse pick (with object snaps)

## Notes

Each segment is committed immediately (undo removes the last segment). Escape cancels the command without removing already-created segments.

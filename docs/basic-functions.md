# Shared game functions in BLIP BASIC

These functions call the same `blip` Rust helpers as the native games. They work
in console expressions, text programs, and game tapes. The console loads the
Rust runtime on the first call and preserves the order of commands entered
while it loads.

| BASIC function | Result |
| --- | --- |
| `MIN(a,b)`, `MAX(a,b)` | Smaller or larger number |
| `ATN(value)` | Arctangent in radians |
| `ATAN2(y,x)` | Direction in radians, including the quadrant |
| `CLAMP(value,low,high)` | Value constrained to the inclusive bounds |
| `LERP(a,b,t)` | Linear interpolation; `t=0` gives `a`, `t=1` gives `b` |
| `WRAP(value,low,high)` | Wrap into `[low,high)`; return `low` if `high<=low` |
| `DISTANCE(x0,y0,x1,y1)` | Distance between two points |
| `ANGLE_DIFF(from,to)` | Shortest signed angle in radians, in `[-π,π)` |
| `SMOOTHSTEP01(t)` | Hermite smoothstep, with input clamped to `[0,1]` |
| `EASE_OUT_CUBIC(t)` | Cubic ease out, with input clamped to `[0,1]` |
| `SCATTER(index,salt)` | Repeatable value in `[0,1)` for scenery |
| `RECTS_OVERLAP(x0,y0,w0,h0,x1,y1,w1,h1)` | `1` if rectangles share area; touching edges give `0` |
| `SEGMENT_CIRCLE_OVERLAP(x0,y0,x1,y1,cx,cy,radius)` | `1` if the segment touches the circle, otherwise `0` |
| `SEGMENT_RECT_OVERLAP(x0,y0,x1,y1,rx,ry,rw,rh)` | `1` if the segment touches the rectangle, otherwise `0` |

Rectangle coordinates describe the top left corner, width, and height.
Interpolation permits values of `t` outside `[0,1]`; the easing functions clamp
their inputs. A segment can have identical endpoints.

Arguments must be numbers and match the listed count. Geometry helpers use the
native games' 32-bit floating point precision; `MIN`, `MAX`, `ATN`, and `ATAN2`
keep BASIC's 64-bit precision. `SCATTER` converts nonnegative
integer arguments to integer types; use values up to 4,294,967,295 for portable
results. Other BASIC arithmetic uses 64-bit numbers.

```basic
10 PLAYER_X=CLAMP(PLAYER_X+DX,8,312)
20 DRAW_X=LERP(PREVIOUS_X,PLAYER_X,FRAME_FRACTION)
30 IF DISTANCE(PLAYER_X,PLAYER_Y,FRUIT_X,FRUIT_Y)<12 THEN GOSUB 100
40 STAR_X=SCATTER(STAR,81)*320
```

Serpent uses the shared clamp and interpolation functions. Adder and Bubbler
use the shared point distance function in both Rust and BASIC.

## Paths

`DRAW_POLYLINE("POINTS",row,count,width,color,alternate)` joins an array's
points in order through the Rust renderer. Fill `POINTS(row,index,0)` with x,
`POINTS(row,index,1)` with y, and optionally `POINTS(row,index,2)` with a marker.
Indices run from 1 to `count`. A segment touching a marked point uses the
alternate color; use the same color twice for an ordinary polyline.

Adder's Rust and BASIC versions use this same renderer for permanent trails
and dim gaps. Other BASIC programs can use it for plots, outlines, or paths.

## Effects

These commands use `blip::Fx`, the same popup, spark, and ring layer used by
the Rust games. Update it once per game step and draw it after the playfield.

| Command | Action |
| --- | --- |
| `FX_CLEAR()` | Remove all effects |
| `FX_UPDATE(seconds)` | Advance motion and lifetimes |
| `FX_DRAW()` | Draw the effects |
| `FX_BURST(x,y,count,speed,color)` | Throw sparks from a point |
| `FX_POPUP(x,y,text,color)` | Show text that rises and fades |
| `FX_RING(x,y,radius,seconds,color)` | Draw an expanding ring |

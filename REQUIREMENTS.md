# Requirements

This document describes the desired features and behavior of the application.


## Summary

The falling glyphs app is like The Matrix in a terminal.


## Components and Behavior

### Glyphs

The codepoint ranges for glyphs is 0x30A0 to 0x30FF

Glyph colors:
  * Leading (bottom) glyph: white
  * Glyphs closer to the head (bottom) of the line should be light green
  * Glyphs closer to the tail (top) of the line should be dark green

  Glyph
  - value: Integer
  - color: GlyphColor

Glyph stacks:
  * A glyph stack is a vertical group of glyphs, which has a random, fixed upper length that is at least one character, and no more than 75% of the height of the terminal.
  * The glyph stack does not move within the viewport. Rather, new, random glyphs are pushed on to the head (bottom) of the stack, and popped off the tail (top) if it becomes too long.
  * There is a 5% chance that any glyph will be altered to another random glyph between updates.
  * Each stack has a different, randomly determined update interval that determines the rate at which new glyphs are added/removed.
  * Stacks are spawned at the top of the viewport, and keep track of x, min_y, and max_y as they are updated. These determine where they are drawn in the viewport.
  * We randomly determine when and where (x value) stacks should be spawned.

  GlyphStack
  - x: Integer
  - min_y: Integer
  - max_y: Integer
  - length: Integer
  - stack: Array[Glyph]
  - update_interval: Duration
  - last_update: Timestamp

### Viewports

We are building this around the concept of a terminal viewport.

We use x,y coordinates for the viewport starting at the upper left. So x_max,y_max is the lower right.

At the start and end of the program, we wipe all content from the terminal viewport.

We maintain a pair of virtual viewports: current and next.

The next viewport is resolved after all Glyph stack states have been resolved.

A diff is computed between current and next viewport to identify changes, then we apply only those changes to the real (terminal) viewport.

Glyph tacks are rendered to the next viewport starting from those having the highest min_y value. The result is an apparent overwrite behavior.

### Controls

Ignore mouse input.

Wipe and exit on ESC, q, Ctrl+D, Ctrl+C


### Flow

Program starts -> screen is wiped -> loop is launched

We have a tick rate that is an lower time limit between updates. 
For each tick:
1. Determine whether any new stacks should be spawned (starts with a single Glyph at the stack's x and y=0)
2. Determine which stacks require an update (update_interval vs. last_update)
3. Update glyph stacks
  3.a. Push a new, white glyph on to the stack
  3.b. Set the prior leading glyph to light green
  3.c. If the internal stack is > length, pop the oldest from the stack
  3.d. Find the middle of the stack, and update that glyph to dark green if it isn't yet
  3.e. Advance the y_min and y_max values
  3.f. If y_min is outside of the viewport, delete the stack
4. Udpate the viewport if it is time (refresh rate is not tied to tick rate)
  4.a. Resolve the next viewport based on the states of all existing stacks
  4.b. Diff the next viewport against the current viewport to generate the patch
  4.c. Apply the patch to the real viewport
  4.d. Replace current with next

Exit key or combo is detected -> screen is wiped -> program exits


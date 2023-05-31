# Time

May 31, 2023

This document discusses alternatives considered to represent musical time. We
need a consistent way to represent musical time that is accurate and convenient
for musical applications.

## Proposal

Store musical time as a single u64 that represents fixed-precision beats.
Provide helper methods that let the experience be similar to Bitwig's
BARs.BEATs.TICKs.% notation.

## Alternatives Considered

- **Samples per second**: for example, if the sample rate is 44.1KHz, then
  something happening at sample 44,100 (zero-indexed) happens at the one-second
  point. This is accurate, but it's not useful for music composition because
  conventional notation is independent of tempo (which means that an increased
  BPM would require all the time markers to be adjusted) and sample rate (which
  means that playback on an output with a different sample rate would be
  incorrect). **Rejected**.
- **MIDI ticks per second**: usually 960. MIDI SMF uses it. **Rejected** for
  same reason as samples per second: it's accurate for representation of a
  static sequence of musical events, but it's not idiomatic for music
  composition.
- **f32/f64 seconds**: this is basically the same as MIDI ticks per second, so
  it's **rejected** for the same reason. It's also very dependent on
  floating-point accuracy/precision, especially if we're comparing an event's
  scheduled time to a clock's time, which means that it will commonly miss edge
  cases.
- **f32/f64 beats**: the single number represents musical beats, so that a piece
  in 4/4 time would play a note with time 4.0 (again, zero-indexed) at the start
  of the second bar. This is better than x-per-second units because it's
  tempo-independent and based on a real musical concept, but like f32/f64
  seconds, it's going to be susceptible to floating-point representation issues.
- **MIDI Manufacturers Association 32-bit Time Cents**: This approach derives a
  time in seconds from 2 ^ ( tc / (1200 x 65536)), where tc is the 32-bit time
  code. A time code is log2(seconds) x 1200 x 65536. The difference between a
  time code of 1 and a time code of 2 is 0.000008628 milliseconds, and the range
  of seconds is from 1 (not zero) to about 60,000 hours. So it's accurate and
  compact, and it far exceeds a useful range for representing the time of a
  piece. But it's a solution to a different problem; like MIDI ticks per second,
  it can't represent musical notation. **Rejected**.
- **Bitwig**: Per [their
  explainer](https://www.bitwig.com/userguide/latest/a_matter_of_timing/), uses
  a BARs.BEATs.TICKs.% notation. Bars are musical bars. Beats are musical beats.
  Ticks are a specified fraction of a note. Percentage covers the time between
  each tick. This approach is nice because it's inherently idiomatic for musical
  composition, and it's accurate in the ways that musical accuracy matters. For
  a BPM 128, 4/4 composition, a single bar is 1.875 seconds, a single beat is
  0.46875 seconds, a single tick is 0.029296875 seconds, and a single percentage
  point is 0.000292969 seconds, or 0.29 milliseconds. If MIDI resolution (960
  ticks/second, or 1.041666667 milliseconds/tick) is good enough, then this is,
  too.
- **Ableton**: "The Arrangement Position fields show the song position in
  bars-beats-sixteenths." It's possible to resize a note in the piano roll to a
  very fine granularity, but I haven't been able to figure out where in the UI
  the actual length is (I see only a "+" on a bar/beat figure that seems to
  indicate it's fractional). It seems reasonable to guess that it's similar to
  Bitwig's BARs.BEATs.TICKs.% notation, but less explicit because it's missing
  the display of the percentage, and I haven't experimented to see whether the
  third value (the sixteenth) is variable based on the time signature.

## Lessons learned from first attempt

Integer representations of the musical units (bars, beats, note values), and a
component that subdivides the smallest integer unit:

```rust
struct MusicalTime {
    bars: u32,  // 0..u32::MAX. Bars are standard (equal to the number of beats
                // in the top of the time signature).

    beats: u8,  // 0..time-signature top. Beats are standard (time-signature
                // top is the number of beats in a bar/measure, and each beat's
                // value is indicated by the time-signature's bottom number).

    parts: u8,  // 0..16. A fraction of a beat.

    subparts: u8, // 0..100. A fraction of a part.
}
```

This is what I did after the first design pass. It works, but it's cumbersome.
It takes something that should be treatable as a single number and makes it a
set of unique individual parts, each of which needs special logic just to do the
things that a normal number should do -- adding, subtracting, carrying on
overflow, etc. It's sort of like an inconsistent BCD.

### New Conclusion

The new version is just a u64. The unit is called a "unit" until I think of a
better name for it. A bar is still made of a variable number of beats, but bar
is virtual (beats / # of beats in bar) rather than being stored as a separate
value. A beat is still made of 16 parts. A part is now 4096 units. This means
that the part can be the bottom 16 bits, and the beats is a super-large 48 bits.

Range: basically infinite. 48 bits of beats is absurdly long.

Resolution: suppose a beat is a quarter note (4/4 time) and the tempo is 120
beats per minute, or two beats per second. This means that a beat lasts for 0.5
seconds, or 500 milliseconds. If we allocate 16 bits to the fractional portion
of a beat, then that gives us a resolution of 1/65536 of a beat, or 0.007629395
milliseconds. Compare MIDI, using 960 parts per quarter note (PPQN), and Bitwig,
using 16 "ticks" per beat, and an integer percentage (0..100) of a tick (1600
PPQN).

I assume that Bitwig chose to divide ticks into 100 parts to make the UI look
more natural. For example, 1.2.15.99 is easy to get used to (1 bar, 2 beats, 15
sixteenths, 99% of the way to the next sixteenth). If we wanted to present the
same UI, then our 1/65536 is broken into 16 sixteenths, and the remaining unit,
which is 1/4096 of a sixteenth, is now rounded to 100 units of about 1/41 of a
sixteenth.

# Time

May 25, 2023

This document discusses alternatives considered to represent musical time. We
need a consistent way to represent musical time that is accurate and convenient
for musical applications.

## Proposal

Model Bitwig's BARs.BEATs.TICKs.% notation, perhaps changing % to fractions of
256.

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

## Conclusion

Do what other DAWs do: have integer representations of the musical units (bars,
beats, note values), and a component that subdivides the smallest integer unit.
Something like this:

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

The `bars` field is the only one that needs to be roomy. For a 4/4 128 BPM piece
(32 bars/minute), a `u16`-sized `bars` field allows a piece to be 65536/32
minutes = 34.13 hours. I could imagine a Philip Glass devotee doing a 35-hour
piece for the laughs, so let's make that at least a `u32`, which allows a 128
BPM piece to be more than 250 years long.

A `beats` size of `u8` allows time signatures from 1/x to 256/x, which include
the common 4/4, 6/8, 9/8, etc.

The `parts` field fits comfortably in a `u8`, and can accommodate as fine as
1/256 of a beat.

Finally, `subparts` should be a `u8`. While it's appealing to use its whole
range 0..255, I suspect Bitwig chose 0..100 because it looks better in the UI. A
base-10 range is more natural to most people than 0..256, even if the latter
offers more granularity.

As for resolution, 4/4 128 BPM works out to 60 / 128 / 4 / 16 / 100 * 1000 =
0.073242188 milliseconds per subpart, and 4/256 time is 0.001144409
msec/subpart. All those are much finer than MIDI's 1.041666667
milliseconds/tick.

If any of these turn out to be too coarse or limited, I think we can bump them
up without a lot of trouble. We're specifying only an in-memory representation,
and the visual representation (4.1.16.99 etc.) is trivially scalable. `subparts`
is unlikely to need further resolution if we can already scale `parts`.

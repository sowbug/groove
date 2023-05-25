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
struct TimeUnit {
    bars: u32,
    beats: u8,
    note_units: u8,
    fraction: u8,

    // 1/note_unit_value, or 1/2^note_unit_value if we needed a few spare bits
    // and were OK restricting to powers of 2.
    // 
    // This should probably live elsewhere, because we don't need every
    // instance to know it.
    note_unit_value: u8, 
}
```

The `bars` field is the only one that needs to be roomy. A `beats` size of `u8`
allows time signatures as fine as 256/x. A `note_units` of `u8` could represent
anything from 1/2 to 1/256 of a beat. Finally, the fraction portion can either
follow Bitwig's lead of a decimal 0..100 that is treated as a percentage, or it
could represent 1/256th of a note_unit. For a 4/4 128 BPM piece (32
bars/minute), a `u16`-sized `bars` allows a piece to be 65536/32 minutes = 34.13
hours. I could imagine a Philip Glass successor doing a 35-hour piece for the
laughs, so let's make that a `u32`, which allows a 128 BPM piece to be more than
250 years long.

As for resolution, 128 BPM works out to a potential resolution of 0.007152557
milliseconds per 1/256-unit fraction of 256th-note units.

If any of these turn out to be too coarse or limited, I think we can bump them
up without a lot of trouble. We're specifying only an in-memory representation,
and the visual representation (4.1.16.255 etc.) is trivially scalable for the
first three parts (as bars/beats are plain numbers, and note_units already
needed a separate unit value accompanying it), and the fourth value is unlikely
to need further resolution if we can already scale `note_units`.

Argument against `fraction` representing 1/256 rather than 1/100: it leads to
surprising UI. 1.1.1.99 isn't 1/100 before 1.1.2.0. Maybe that's why Bitwig went
with a base-10 system for that component.

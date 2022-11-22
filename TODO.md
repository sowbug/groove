# TODO

## Architecture proofs

- [ ] Solve whether refcounting/interior mutability is required. This is a
  prerequisite for scaling up effects/instruments. Refcounting is especially
  needed for embedded/no_std.
- [x] An external thread (MIDI input) can inject events
- [ ] Live play: keyboard plays sounds, patterns can play one-off or loop on
  command
- [ ] Interactive code editing: we can update the project during play,
  gracefully
- [ ] Different types of devices can render in terms of other things (e.g.,
  automation as a layer on top of the thing it controls)
- [ ] Different views of same device: expanded, collapsed, detail, summary,
  enabled, disabled, playing
- [ ] Organized view: it's clear what makes an audio "lane" work. This might be
  in terms of audio sources (with orphans) or MIDI routes.
- [ ] What's the real engineering cost of adding a new device?
- [ ] Verify that capable instruments can generate same events
  backwards/forwards.

## Table stakes

- [ ] Stereo
- [ ] Output to MP3
- [X] MIDI input
- [X] MIDI output
- [ ] Swing
- [ ] Interactivity: I shouldn't have to keep pressing play and listening to the
  whole song

## Batteries included

- [ ] Instrument: FM synth
- [ ] Instrument: tunable sampler
- [ ] Effect: reverb
- [ ] Effect: chorus
- [ ] Effect: ping-pong
- [ ] Effect: compression
- [ ] Effect: arithmetic operations - add (mix), subtract, add a constant.. what
  else?
- [ ] Effect: delay
- [ ] Effect: 24db filters
- [ ] MIDI Instrument: Arpeggiator
- [ ] Wavetables instead of algorithmically generated waveforms
- [ ] Sidechaining
- [ ] Complete Welsh library

## Advanced features

- [ ] VST, Clap support
- [ ] Waveforms that have mass: you steer them, but they lag your inputs

## Creative experience

- [ ] GUI: each block in the serialization has a widget
- [ ] Automation triggers
- [ ] Filters: linear-to-log mapping for Q
- [ ] Revisit scripting
- [ ] Generate a random EDM song as a new-project template
- [ ] Visualize time domain
- [ ] Visualize frequency domain
- [ ] Make it easier to recover from source/syntax errors. For example,
  references to missing IDs, or IDs used in the wrong context
- [ ] Source modules, namespaces
- [ ] Story for collaboration
- [ ] A converter for MIDI SMF -> native
- [ ] Other notation formats: not everything wants to be a pattern
- [ ] GUI: Sandbox to hear snippets of source

## Misc applications

- [ ] Could it also be a game sound engine?

## Code health, performance

- [ ] Optimization: memoization (see
  <https://www.textualize.io/blog/posts/7-things-about-terminals>)
- [ ] Universal time unit during scheduling
- [x] Universal time unit during rendering (it's sample, as in sample rate)
- [ ] Unit test the filters
- [ ] More unit tests
- [x] Generalize envelopes
- [x] Generic unit tests for all the traits
- [ ] Maybe let audio processors work in wider float ranges, and then clamp only
  at the end of the chain
- [ ] Regularly test with weird sample rates (super low, super high)
- [ ] Identify entities that don't need ticks, and don't tick() them
- [x] Come up with a better TODO than `panic!()`. Get comfortable with handling
  `Result<>`.

## Random thoughts

- [ ] Is SourcesAudio actually multiple things? Some things like Oscillators are
  pure functions, mostly functions of time. Others, like a filter, maintain lots
  of internal state and can't be accessed randomly. While a distinction
  expressed in terms of traits might not change the program flow very much,
  having the knowledge of which is which might allow some optimizations later
  down the road.

## Bugs

- [ ] When scrubbing, MIDI instruments should turn off any playing instruments
  that would have been turned off during the skipped part.

## Project logistics

- [ ] Automatic releases
- [ ] Try on different platforms
- [ ] Documentation
- [ ] Pick a file extension for project files
- [ ] Make a [JSON
  schema](https://dev.to/brpaz/how-to-create-your-own-auto-completion-for-json-and-yaml-files-on-vs-code-with-the-help-of-json-schema-k1i)
  for editors to handle autocompletion
- [ ] Pick a serialization format. TOML, YAML, JSON...

## Scrapbook

```text
gain-1         : 7
bassline       : 3
main-mixer     : 1
piano-1        : 2
low-pass-1     : 11
arp-1          : 6
synth-1        : 4
trip-1         : 13
gain-2         : 8
gain-3         : 9
bitcrusher-1   : 10
drum-1         : 5
    stack.push(StackEntry::ToVisit(self.main_mixer_uid=1));
LOOP #0
stack.pop() -> ToVisit(1)
    source_audio(5)
                LOOP #1
                stack.pop() -> ToVisit(9)
                    source_audio(4)
                    LOOP #2
                    stack.pop() -> Result 0
                LOOP #3
                stack.pop() -> CollectResultFor(9)
                transform_audio(9)
                LOOP #4
                stack.pop() -> Result -0
            LOOP #5
            stack.pop() -> ToVisit(8)
                source_audio(3)
                LOOP #6
                stack.pop() -> Result 0
            LOOP #7
            stack.pop() -> CollectResultFor(8)
            transform_audio(8)
            LOOP #8
            stack.pop() -> Result -0
        LOOP #9
        stack.pop() -> ToVisit(7)
                LOOP #10
                stack.pop() -> ToVisit(11)
                        LOOP #11
                        stack.pop() -> ToVisit(10)
                            source_audio(2)
                            LOOP #12
                            stack.pop() -> Result 0
                        LOOP #13
                        stack.pop() -> CollectResultFor(10)
                        transform_audio(10)
                        LOOP #14
                        stack.pop() -> Result -0
                    LOOP #15
                    stack.pop() -> Result 0
                LOOP #16
                stack.pop() -> CollectResultFor(11)
                transform_audio(11)
                LOOP #17
                stack.pop() -> Result -0
            LOOP #18
            stack.pop() -> Result 0
        LOOP #19
        stack.pop() -> CollectResultFor(7)
        transform_audio(7)
        LOOP #20
        stack.pop() -> Result -0
    LOOP #21
    stack.pop() -> Result 0
LOOP #22
stack.pop() -> CollectResultFor(1)
transform_audio(1)
LOOP #23
stack.pop() -> Result 0
```

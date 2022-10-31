# TODO

## Architecture proofs

- [ ] Is refcounting/interior mutability required?
- [x] An external thread (MIDI input) can inject events
- [ ] We can update the project during play, gracefully
- [ ] Different types of devices can render in terms of other things (e.g., automation as a layer on top of the thing it controls)
- [ ] Different views of same device: expanded, collapsed, detail, summary, enabled, disabled, playing
- [ ] What's the real engineering cost of adding a new device?

## Table stakes

- [ ] Stereo
- [ ] Output to MP3
- [ ] MIDI input
- [ ] MIDI output
- [ ] Swing
- [ ] Interactivity: I shouldn't have to keep pressing play and listening to the whole song

## Batteries included

- [ ] Instrument: FM synth
- [ ] Instrument: tunable sampler
- [ ] Effect: reverb
- [ ] Effect: chorus
- [ ] Effect: ping-pong
- [ ] Effect: compression
- [ ] Effect: arithmetic operations - add (mix), subtract, add a constant.. what else?
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
- [ ] Make it easier to recover from source/syntax errors. For example, references to missing IDs, or IDs used in the wrong context
- [ ] Source modules, namespaces
- [ ] Story for collaboration
- [ ] A converter for MIDI SMF -> native
- [ ] Other notation formats: not everything wants to be a pattern
- [ ] GUI: Sandbox to hear snippets of source

## Misc applications

- [ ] Could it also be a game sound engine?

## Code health, performance

- [ ] Optimization: memoization (see https://www.textualize.io/blog/posts/7-things-about-terminals)
- [ ] Universal time unit during scheduling
- [x] Universal time unit during rendering (it's sample, as in sample rate)
- [ ] Unit test the filters
- [ ] More unit tests
- [ ] Generalize envelopes
- [ ] Generic unit tests for all the traits
- [ ] Maybe let audio processors work in wider float ranges, and then clamp only at the end of the chain
- [ ] Regularly test with weird sample rates (super low, super high)
- [ ] Identify entities that don't need ticks, and don't tick() them

## Random thoughts

- [ ] Is SourcesAudio actually multiple things? Some things like Oscillators are pure functions, mostly functions of time. Others, like a filter, maintain lots of internal state and can't be accessed randomly. While a distinction expressed in terms of traits might not change the program flow very much, having the knowledge of which is which might allow some optimizations later down the road.

## Project logistics

- [ ] Automatic releases
- [ ] Try on different platforms
- [ ] Documentation
- [ ] Pick a file extension for project files
- [ ] Make a [JSON schema](https://dev.to/brpaz/how-to-create-your-own-auto-completion-for-json-and-yaml-files-on-vs-code-with-the-help-of-json-schema-k1i) for editors to handle autocompletion

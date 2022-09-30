# TODO

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

## Project logistics

- [ ] Automatic releases
- [ ] Try on different platforms
- [ ] Documentation
- [ ] Pick a file extension for project files
- [ ] Make a [JSON schema](https://dev.to/brpaz/how-to-create-your-own-auto-completion-for-json-and-yaml-files-on-vs-code-with-the-help-of-json-schema-k1i) for editors to handle autocompletion
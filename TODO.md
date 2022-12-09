# TODO

## Architecture proofs

- [x] Solve whether refcounting/interior mutability is required. This is a
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
- [x] What's the real engineering cost of adding a new device?
- [ ] Create a definition of random-access capability and write tests to enforce
  it. The point is that everything should behave well if the user skips around
  in the GUI during playback. To a lesser extent, well-defined behavior in this
  respect will also allow more parallelization across CPU cores later on.

## Table stakes

- [ ] Stereo
- [ ] Output to MP3
- [X] MIDI input
- [X] MIDI output
- [ ] Swing
- [ ] Interactive playback: I shouldn't have to keep pressing play and listening
  to the whole song
- [ ] Interactive recording: It should be easier to get new data into a project

## Batteries included

- [ ] Instrument: FM synth
- [ ] Instrument: tunable sampler
- [ ] Instrument: Wavetables instead of algorithmically generated waveforms
- [ ] Instrument: Complete Welsh library
- [ ] Controller: Arpeggiator
- [ ] Controller: Sidechaining
- [ ] Effect: chorus
- [ ] Effect: ping-pong
- [ ] Effect: compression
- [ ] Effect: arithmetic operations - add (mix), subtract, add a constant.. what
  else?
- [ ] Effect: 24db filters
- [x] Effect: reverb
- [x] Effect: delay
- [ ] GUI: a browser for everything, allowing quick demoing and instantiation
- [ ] Infrastructure: it should be easy to pull other files into your project
  source

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
- [ ] GUI: Sandbox to hear snippets of source (don't go overboard; most people
  will use VSCode or their favorite IDE)

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

## Ideas

- [ ] .

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

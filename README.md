# Groove

A digital audio workstation (DAW) engine.

## The big vision

I'm a software engineer in my day job. I also dabble in music production. I find
the collaborative workflow in major DAWs today to be confusing. People more or
less email around project files. So sharing a project is all-or-nothing. There
isn't any [source or version
control](https://en.wikipedia.org/wiki/Version_control). In fact, there isn't
even any source in the way software engineers usually think of it. If you wanted
to show someone else how you did something, the state of the art appears to be
typing out what you did in English ("select the 24db low-pass filter, then drag
the thingie until it sounds like bwaaaap") or making a 10-minute YouTube video
to show 10 seconds of mouse clicks. And if you want to checkpoint a version of
your song project, you quit your DAW, make a copy of the project file(s), and
rename the copy. It's crude and error-prone.

I know that coding and music production are different, and the workflows
_should_ be different. Moreover, the businesses are very different, and the
notion of "open-source music" isn't as common as "open-source software." I don't
have a problem with this difference, but I'd like to see whether a solution to a
software-engineering problem could also apply to music engineering/production
(and whether using that solution would be attractive to at least some music
producers).

I imagine a desktop app with two side-by-side panels. The right looks a lot like
a modern DAW: an arrangement view with horizonal tracks, and a detail view
showing effect chains and spectrum analyzers. The left looks a lot like a
software IDE: it's a bunch of tabs with indented code. When you move a dial or
drag a pattern on the right side of the screen, a block of code on the left side
of the screen is highlighted, and parts of it might even update automatically to
stay consistent. Likewise, editing the code on the left causes immediate changes
to the GUI representation on the right. Either panel is optional to get the job
done, but if you find it easier to express an idea in text vs. graphical
widgets, you can do so without breaking your flow.

The text on the left is the truth. It's what you save when you save your project
file. And if you want to share a technique with someone, it might be as simple
as pasting a few lines of text into a messaging app. If you want to merge two
versions of a collaborative project, you can use [any of the many excellent
tools](https://en.wikipedia.org/wiki/Comparison_of_file_comparison_tools) that
exist for that purpose. You can check your song into Git and know that the
commit diffs will always be meaningful. And for larger projects that include
songs as part of their media, hopefully this style will fit better into their
version-controlled workflows.

## High-level project status

- Producing a song is possible only with the CLI, as the GUI doesn't allow
  editing. I expect it would be a tedious experience, to say the least.
- Aside from the workflow being difficult, there aren't many components -- just
  a couple instruments, a few effects, and a controller (automation) system. So
  even if you liked the workflow, you don't have a rich library of tools to work
  with.
- A proof-of-concept GUI exists.

## Current Features

- Simple subtractive synthesizer. Dual oscillators with a low-pass filter and
  LFO. Design target is to properly implement the patches listed in [Welsh's
  Synthesizer
  Cookbook](https://www.amazon.com/Welshs-Synthesizer-Cookbook-Programming-Universal/dp/B000ERHA4S/),
  3rd edition, by Fred Welsh ([website](https://synthesizer-cookbook.com/)).
- Sampler. Doesn't yet know anything about tones; in other words, it just plays
  back WAV data at the original speed, which works fine for a drumkit but not so
  well for tonal sounds that you expect to use melodically.
- Sequencer with a MIDI SMF reader.
- A declarative project language, which makes it easy to produce songs in YAML
  or JSON format (JSON only in theory, but we get it for free thanks to
  [serde](https://serde.rs/)).
- A few audio effects (gain, limiter, bitcrusher, filters).
- Basic automation.
- Output to WAV file or speaker.
- A very simple [Iced](https://iced.rs/)-based GUI, which isn't useful for much
  of anything yet, but has been useful to keep the architecture something that
  can eventually be integrated with a GUI.
- Plenty of bugs.

## On the roadmap

- More of everything: synths, filters, effects. High-priority gaps to fill are a
  wavetable synth, an FM synth, a proper sampler, 24db filters, and must-have
  effects like chorus, reverb, delay, etc.
- A better automation design. The vision is intuitively configurable envelopes
  that can be used throughout the system.
- Scripting. Currently I'm experimenting with [rhai](https://rhai.rs/), but I
  don't know whether a JavaScript-y language is right for this domain.
- MIDI input/output.
- A GUI that at a minimum provides visual feedback on the project (read-only),
  and ideally also allows editing of the project (interactive).
- Audio tools. Visualizing audio in a frequency-domain format is top priority.
- Better sound quality. Since the basic oscillators are generated using pure
  algorithms (sine, sawtooth, square, etc.), they suffer from aliasing and
  unwanted transients. The efficient state-of-the-art solution these days seems
  to be to generate them offline and delegate to wavetable synthesis.
- Plugin support (VST, [CLAP](https://u-he.com/community/clap/)). I think it's
  possible to carry out the vision even if components of a project are
  closed-source. If a plugin's persistent state is a set of values that can be
  represented in a config file, then it should fit in.
- Assistance with song composition. Suggesting pleasant chord progressions,
  identifying the current scale and highlighting the full set of consonant
  notes, a [hyperspace](https://en.wikipedia.org/wiki/Asteroids_(video_game))
  button to suggest new directions, etc.
- A review of other DAW features, then playing catchup.

## Other projects/resources of interest

- [MiniDexed](https://github.com/probonopd/MiniDexed)
- [minisynth](https://github.com/rsta2/minisynth)
- [Dirtywave M8](https://dirtywave.com/)
- [BasicSynth](https://basicsynth.com/)
- [Welsh's Synthesizer Cookbook](https://synthesizer-cookbook.com/)
- [SynthLab](https://www.willpirkle.com/synthlab-landing/)
- [Glicol](https://github.com/chaosprint/glicol) is consistent with the vision.
- [Sonic Pi](https://sonic-pi.net/), which I somehow missed until just now.

## Installation for development

I use VSCode for development.

- `curl https://sh.rustup.rs -sSf | sh`
- `rustup default nightly` (until [trait
  upcasting](https://doc.rust-lang.org/beta/unstable-book/language-features/trait-upcasting.html)
  is stable)
- `apt install pkg-config libasound2-dev libfontconfig-dev`

## Coding conventions (WIP, subject to change and caprice)

- For structs that exist primarily because of traits, the trait implementations
  should come first, and then the struct-specific implementations should come
  after that.

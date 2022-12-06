# The vision for this project

I'm a software engineer. I also dabble in music production. I love software
engineering's collaborative tools. I also love music production's creative
tools. I think that a workflow that merges these collaborative and creative
tools will be a good combination.

Software engineers have developed a sixth sense that I'll call **the itch**.
They feel this itch when they haven't "checked in" to "the repo" for a while.
"Checking in" means updating your copy of your team's project, and publishing
your work to the rest of the team. Engineers exchange feedback by checking in.
It's how they communicate.

Checking in is central to the life of a software engineer. It's no surprise that
we have great check-in tools. These tools make it easy to compare your work with
the team's shared copy. They help you track down exactly when a cool feature or
bug first appeared. If you feel like experimenting, these tools let you "fork"
or "branch" your copy of the project, mess around with it, rewind if things go
sideways, and finally pick and choose the good parts of your experiment to share
with the rest of your team. Software engineering isn't always fun, but I've
always loved the so-called "change management" part of it. Seeing the flow of
information across the whole team gives me a buzz. And having the chance to see
and learn from everyone else's work, as it's happening, certainly improves my
own skills and work.

Unfortunately, my experience with music production isn't quite the same.
Collaboration means emailing an Ableton Live project file, or watching a
10-minute YouTube video to learn that someone turned one knob in a DAW to make a
certain sound. Experimenting with a song means quitting the DAW, making a copy
of the project file, and hoping I don't mix up the old and new copies. I am much
more experienced as a software engineer than as a music producer, but I think
these differences are real.

I concede that software development and music production are not identical.
They're different disciplines in different industries. Industry money moves
differently; songs and software projects are usually very different sizes, which
calls for very different team sizes; and the line between a software product and
a software tool is very fuzzy compared to the line between a song and an
instrument. It follows that tooling and workflows should be different. But I
still think better music, and more music, can be made by more people working
together in an environment that facilitates collaboration the way software
engineering does.

So what's actually different about this project? It's still a DAW. You still sit
down at your desk with an idea and end up with an MP3 you send to your friends.
But instead of interacting with the DAW GUI's knobs and dials, you'll also have
the option of working directly with the human-readable "source code" that
produces your song, and that's where music production starts to look more like
software engineering.

Imagine a two-panel desktop app. The left side looks like an IDE ("integrated
development environment," which is a very smart Notepad app that software
engineers use to write their code). The right side looks like a DAW ("digital
audio workstation," which is like Photoshop but for music). The left side is a
text representation of the song. The right side is the same, but graphical.
Making a change to the left side, for example changing `gain: 0.5` to `gain:
0.6`, changes one of the right side's track volume knobs from 50% to 60%.
Similarly, adjusting that volume knob on the right side offers to update the
corresponding source code on the left side to stay consistent.

So far, we have a standard DAW that happens to have a relatively easy-to-read
project-file format that you can edit real-time. Not a huge difference. It'll
also be possible to author a song using a more "imperative" style than the
static "declarative" style of a typical app file format. That means writing
something like JavaScript to create a song. Depending on your personal
experience, that might be a dream or a nightmare. But it's an option if you want
it. And once songs are scriptable, a lot is possible after that.

If this working style resonates with enough people, then some things will change
in the future:

- More songs will be published with source code.
- Songs will have more remixes by people with little experience in music
  production.
- There will be more informal collaboration in music production, as people post
  tiny snippets of source code online to get feedback from people they've never
  met.
- It'll be easier to get started in music production, because you can find the
  source to a song you like, twiddle it line-by-line, and learn by trial and
  error what each line does.
- More of the audio assets of a larger project, such as a video game, or a
  commerical, or a movie, will fit in with existing project workflows and their
  source-code management. (*Warning:* I'm reasoning from first principles with
  this point. The closest I've worked to a project like a movie is certain kinds
  of media-heavy software projects. We still contracted with an audio designer
  who sent us .WAV files as the sole representation of the contract work, and
  years later we had no idea how to re-create those sound files because we
  didn't even know which tool the original contractor used to make them, let
  along having the project files that went along with the tool. For all I know,
  today's state of the art process already captures the source code and tooling
  in the aggregate project's build steps.)
- Certain kinds of songs (especially EDM based on synthesized rather than
  sampled sound generation) will be representable in a couple kilobytes of text.
  This doesn't solve a problem that anyone actually has, but I think it's cool.

My favorite fantasy outcome is that instead of creating new songs from scratch,
producers branch another song's project file, so that new songs are part of this
"family tree" of music. We'll be able to trace a song's ancestry, recognizing
the many people who contributed to its lineage over the years. Most new music is
heavily influenced by existing music and thousands of years of evolving styles.
This workflow change incorporates that evolution into a song's explicit
documentary history.

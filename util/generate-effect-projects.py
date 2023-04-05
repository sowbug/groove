#!/usr/bin/env python3

EFFECTS = [
    {'effect-name': 'filter-low-pass-12db',
     'waveforms': ['noise', 'sine'],
     'configs': [{'cutoff': 1000.0, 'q': 0.707},
                 {'cutoff': 1000.0, 'q': 20.0},
                 ]
     },
    {'effect-name': 'filter-high-pass-12db',
     'waveforms': ['noise', 'sine'],
     'configs': [{'cutoff': 1000.0, 'q': 0.707},
                 {'cutoff': 1000.0, 'q': 20.0},
                 ]
     },
    {'effect-name': 'filter-band-pass-12db',
     'waveforms': ['noise', 'sine'],
     'configs': [{'cutoff': 1000.0, 'bandwidth': 30.0},
                 {'cutoff': 1000.0, 'bandwidth': 2000.0},
                 ]
     },
    {'effect-name': 'filter-band-stop-12db',
     'waveforms': ['noise', 'sine'],
     'configs': [{'cutoff': 1000.0, 'bandwidth': 30.0},
                 {'cutoff': 1000.0, 'bandwidth': 2000.0},
                 ]
     },
    {'effect-name': 'filter-all-pass-12db',
     'waveforms': ['noise', 'sine'],
     'configs': [{'cutoff': 1000.0, 'q': 0.707},
                 {'cutoff': 1000.0, 'q': 20.0},
                 ]
     },
    {'effect-name': 'filter-peaking-eq-12db',
     'waveforms': ['noise', 'sine'],
     'configs': [{'cutoff': 1000.0, 'db-gain': 6.0},
                 {'cutoff': 1000.0, 'db-gain': 30.0},
                 ]
     },
    {'effect-name': 'filter-low-shelf-12db',
     'waveforms': ['noise', 'sine'],
     'configs': [{'cutoff': 1000.0, 'db-gain': 6.0},
                 {'cutoff': 1000.0, 'db-gain': 30.0},
                 ]
     },
    {'effect-name': 'filter-high-shelf-12db',
     'waveforms': ['noise', 'sine'],
     'configs': [{'cutoff': 1000.0, 'db-gain': 6.0},
                 {'cutoff': 1000.0, 'db-gain': 30.0},
                 ]
     },
    {'effect-name': 'bitcrusher',
     'waveforms': ['sawtooth', 'triangle'],
     'configs': [{'bits-to-crush': 8},
                 {'bits-to-crush': 13},
                 ]
     },
    {'effect-name': 'limiter',
     'waveforms': ['noise', 'sine'],
     'configs': [{'min': 0.1, 'max': 0.9},
                 {'min': 0.4, 'max': 0.6},
                 ]
     },
    {'effect-name': 'gain',
     'waveforms': ['noise', 'sine'],
     'configs': [{'ceiling': 0.1},
                 {'ceiling': 0.5},
                 ]
     },
]

TEMPLATE = '''---
title: "{description}"
clock:
  bpm: 240.0
  time-signature:
    - 4
    - 4
devices:
  - instrument:
    - instrument-1
    - oscillator:
      - midi-in: 0
        waveform: {waveform}
        frequency: {frequency}
  - effect:
    - effect-1
    - {effect_name}:
        {params}
patch-cables:
  - [instrument-1, effect-1, main-mixer]
patterns:
  - id: silent-1
    notes:
      - [0]
tracks:
  - id: track-1
    midi-channel: 0
    patterns: [silent-1]
'''


def do_it(effect_name, waveform, config):
    config_escaped = []
    for k, v in config.items():
        if v < 1.0:
          config_escaped.append("%s-%0.3f" % (k, v))
        else:
          config_escaped.append("%s-%d" % (k, v))
    description = "%s_%s_%s" % (
        effect_name, waveform, "_".join(config_escaped))
    filename = description + ".yaml"
    with open(filename, "w") as f:
        params = []
        for k, v in config.items():
            params.append("%s: %s" % (k, v))
        params = "\n        ".join(params)
        if waveform == 'noise':
            frequency = 0.0
        else:
            frequency = 440.0
        f.write(TEMPLATE.format(description=description,
                effect_name=effect_name, waveform=waveform, frequency=frequency, params=params))


for effect in EFFECTS:
    for waveform in effect['waveforms']:
        for config in effect['configs']:
            do_it(effect['effect-name'], waveform, config)

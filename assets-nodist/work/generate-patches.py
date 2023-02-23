#!/usr/bin/python3

import os
import csv
from oyaml import dump


def as_pct(s):
    if s == '':
        return 0.0
    else:
        return float(s.rstrip("%")) / 100.0


def as_cents(s):
    if s == '':
        return 0.0
    else:
        return float(s.rstrip(" cents"))


def as_int(s):
    if s == '':
        return 0
    else:
        return int(s.replace(',', ''))


def as_bool(s):
    if s is None:
        return False
    if s == '':
        return False
    if s.lower() == 'false':
        return False
    return bool(s)


def as_float(s):
    if s == '':
        return 0
    else:
        return float(s)


def as_envelope(s, what_is_max):
    if s == '':
        return 0.0
    if s == 'max':
        # For decay, this happens only when sustain is 100%, so it doesn't
        # matter
        #
        # For release, this happens only on filter envelope, where max should be
        # a very long time (30+ seconds) to outlast amp envelope
        return what_is_max
    return float(s)


def as_tune(o, s, c, n):
    if n != '':
        return {"note": int(n)}
    o = as_int(o)
    s = as_int(s)
    c = as_int(c)
    if o == 0 and s == 0 and c == 0:
        return {'float': 1.0}
    return {
        'osc': {
            'octave': o,
            'semi': s,
            'cent': c
        }
    }


def as_depth(p, c):
    if p is None or c is None:
        return 'none'
    if p == '10/17%':
        p = '10%'  # Galactic Chapel
    p = as_pct(p)
    c = as_cents(c)
    if p != 0.0 and c == 0:
        return {'pct': p}
    if p == 0.0 and c != 0:
        return {'cents': c}
    return 'none'


def as_kebab(s):
    return s.lower().replace(" ", "-").replace(",", "").replace("/", "-")


def as_waveform(s):
    if s.startswith("PW"):
        return {"pulse-width": as_pct(s[3:])}
    if s == '':
        return "none"
    return as_kebab(s)


def as_polyphony(s):
    try:
        n = int(s)
        if n > 0:
            return {"multi-limit": n}
    except:
        return as_kebab(s)


def as_routing(s):
    if s == '':
        return "none"
    return as_kebab(s)


def as_glide(s):
    if s == "\"moderate\"":
        s = 0.1
    return as_float(s)


with open("patches.csv") as csvfile:
    reader = csv.reader(csvfile)
    for row in reader:
        if row[0].startswith("Exception"):
            continue
        if row[1] == "103":
            continue
        name = as_kebab(row[2])

        patch = {
            'name': name,
            'oscillator-1': {
                'waveform': as_waveform(row[3]),
                'tune': as_tune(row[4], row[5], row[6], ''),
                'mix-pct': as_pct(row[7]),
                'mix-db': as_float(row[8]),
            },
            'oscillator-2': {
                'waveform': as_waveform(row[9]),
                'tune': as_tune(row[11], row[12], row[13], row[10]),
                'mix-pct': as_pct(row[14]),
                'mix-db': as_float(row[15]),
            },
            'oscillator-2-track': as_bool(row[16]),
            'oscillator-2-sync': as_bool(row[17]),
            'noise': as_pct(row[19]),
            'lfo': {
                'routing': as_routing(row[20]),
                'waveform': as_waveform(row[21]),
                'frequency': as_float(row[22]),
                'depth': as_depth(row[23], row[24]),
            },
            'glide': as_glide(row[26]),
            'unison': as_bool(row[27]),
            'polyphony': as_polyphony(row[28]),
            'filter-type-24db': {
                'cutoff-hz': as_int(row[29]),
                'cutoff-pct': as_pct(row[30]),
            },
            'filter-type-12db': {
                'cutoff-hz': as_int(row[31]),
                'cutoff-pct': as_pct(row[32]),
            },
            'filter-resonance': as_pct(row[33]),
            'filter-envelope-weight': as_pct(row[34]),
            'filter-envelope': {
                'attack': as_float(row[35]),
                'decay': as_envelope(row[36], 0.0),
                'sustain': as_pct(row[37]),
                'release': as_envelope(row[38], 30.0),
            },
            'amp-envelope': {
                'attack': as_float(row[39]),
                'decay': as_envelope(row[40], 0.0),
                'sustain': as_pct(row[41]),
                'release': as_envelope(row[42], None),
            },
        }
        with open("../../assets/patches/welsh/%s.yaml" % name, "w") as patchfile:
            patchfile.write(dump(patch, explicit_start=True))

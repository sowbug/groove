#!/usr/bin/python3

import os
import csv
import pprint


def as_pct(s):
    if s == '':
        return 0.0
    else:
        return float(s.rstrip("%"))


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


def as_float(s):
    if s == '':
        return 0
    else:
        return float(s)


def as_envelope(s):
    if s == '':
        return 0.0
    if s == 'max':
        return 99999.9
    return float(s)


def as_tune(o, s, c, n):
    if n != '':
        return {"note": n}
    o = as_int(o)
    s = as_int(s)
    c = as_int(c)
    if o == 0 and s == 0 and c == 0:
        return None
    return {
        'octave': o,
        'semi': s,
        'cent': c
    }

def as_depth(p, c):
    p = as_pct(p)
    c = as_cents(c)
    if p != 0.0 and c == 0:
        return { 'pct': p }
    if p == 0.0 and c != 0:
        return { 'cents': c }
    return None

with open("patches.csv") as csvfile:
    reader = csv.reader(csvfile)
    for row in reader:
        patch = {
            'name': row[2],
            'osc_1': {
                'wave': row[3],
                'tune': as_tune(row[4], row[5], row[6], ''),
                'mix_pct': as_pct(row[7]),
                'mix_db': as_float(row[8]),
            },
            'osc_2': {'wave': row[9],
                      'tune': as_tune(row[11], row[12], row[13], row[10]),
                      'mix_pct': as_pct(row[14]),
                      'mix_db': as_float(row[15]),
                      'track': bool(row[16]),
                      'sync': bool(row[17]),
                      },
            'noise': as_pct(row[19]),
            'lfo': {
                'routing': row[20],
                'wave': row[21],
                'hz': as_float(row[22]),
                'depth': as_depth(row[23], row[24]),
            },
            'glide': as_float(row[26]),
            'unison': bool(row[27]),
            'voices': row[28],
            'filter_24db': {
                'hz': as_int(row[29]),
                'pct': as_pct(row[30]),
            },
            'filter_12db': {
                'hz': as_int(row[31]),
                'pct': as_pct(row[32]),
            },
            'filter': {
                'resonance_pct': as_pct(row[33]),
                'envelope_pct': as_pct(row[34]),
                'attack': as_float(row[35]),
                'decay': as_envelope(row[36]),
                'sustain': as_pct(row[37]),
                'release': as_envelope(row[38]),
            },
            'amp_envelope': {
                'attack': as_float(row[39]),
                'decay': as_envelope(row[40]),
                'sustain': as_pct(row[41]),
                'release': as_envelope(row[42]),
            },
        }
        pprint.pprint(patch)

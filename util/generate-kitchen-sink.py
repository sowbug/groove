import random
from torch import float_power
import yaml

devices = []
patch_cables = []
patterns = []
tracks = []
paths = []
trips = []

trip_id = 1
def get_trip_id():
    global trip_id
    s = "trip-%d" % (trip_id)
    trip_id += 1
    return s

float_param = 0.01
def get_float_param():
    global float_param
    p = float_param
    float_param += 0.01
    if float_param > 0.9:
        float_param = 0.01
    return int(p * 100) / 100.0

def add_trip(target_id, param, measure, paths):
    trips.append({'id': get_trip_id(), 'target': {'id': target_id, 'param': param}, 'start-measure': measure, 'paths': paths})

EFFECTS = {
    'gain': ['ceiling'],
    'limiter': ['min', 'max'],
    'bitcrusher': ['bits-to-crush'],
    'filter-low-pass-12db': ['cutoff', 'q'],
    'filter-high-pass-12db': ['cutoff', 'q'],
    'filter-band-pass-12db': ['cutoff', 'bandwidth'],
    'filter-band-stop-12db': ['cutoff', 'bandwidth'],
    'filter-all-pass-12db': ['cutoff', 'q'],
    'filter-peaking-eq-12db': ['cutoff', 'db-gain'],
    'filter-low-shelf-12db': ['cutoff', 'db-gain'],
    'filter-high-shelf-12db': ['cutoff', 'db-gain'],
}
for k, v in EFFECTS.items():
    params = {}
    for param in v:
        if param == 'bits-to-crush':
            params[param] = 8
        else:
            params[param] = get_float_param()
    li = ['%s-1' % (k), {k: params}]
    dict = {'effect': li }
    devices.append(dict)

for k, v in EFFECTS.items():
    for param in v:
        add_trip('%s-1' % (k), param, 7, ['auto-1'])

d = {
    'clock':
    {
        'sample-rate': 44100,
        'bpm': 128.0,
        'time-signature': [4, 4]
    },
    'devices':devices,
    'patch-cables':patch_cables,
    'patterns':patterns,
    'tracks':tracks,
    'paths':paths,
    'trips':trips
}

print(yaml.dump(d))

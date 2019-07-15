#!/usr/bin/env python3

import sys
import csv

with open(sys.argv[1], 'r') as f:
    reader = csv.reader(f, delimiter=',')
    for i, line in enumerate(reader):
        if i == 0:
            del line[1]
            print(','.join(line))
        else:
            baseline = float(line[1])
            del line[1]
            print(line[0] + ',' + ','.join([str(float(x) / baseline) for x in line[1:]]))

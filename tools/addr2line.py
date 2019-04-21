#!/usr/bin/env python3
import sys
import re
import subprocess

print('Paste backtrace here, and then input EOF(Ctrl-D or Ctrl-Z) to get annotated backtrace.')
lines = sys.stdin.readlines()
addrline = sys.argv[1]
arch = sys.argv[2]
mode = sys.argv[3]
print('--------------------------------------')
for line in lines:
    match = re.search('#([0-9]+) PC: (0x[0-9A-F]+) FP: (0x[0-9A-F]+)', line)
    if match:
        addr = match.group(2)
        process = subprocess.run([addrline, '-e', 'target/{0}/{1}/rcore'.format(arch, mode), '-f', '-C', addr], capture_output=True)
        res = process.stdout.decode('utf-8')
        print('#{0} PC: {1} FP: {3} {2}'.format(match.group(1), match.group(2), res.strip(), match.group(3)))
    else:
        print(line, end='')

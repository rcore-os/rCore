#!/bin/bash
for f in *.exp
do
    echo run $f
    timeout 30s expect $f
done

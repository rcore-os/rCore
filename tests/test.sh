#!/bin/bash
for f in *.cmd
do
    echo testing $f begin
    (
        cd ../kernel
        exec timeout 10s make runtest arch=riscv64 init=$(cat ../tests/$f)
    ) &

    pid=$!

    wait $pid

    diff -I 'bbl loader' -I 'Hello RISCV! in hart' -u ${f%.cmd}.out stdout || { echo 'testing failed for' $f; exit 1; }

    echo testing $f pass
done

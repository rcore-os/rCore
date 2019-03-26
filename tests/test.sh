#!/bin/bash
cd ../kernel && make sfsimg arch=riscv32 && cd ../tests
for f in *.cmd
do
    echo testing $f begin
    (
        cd ../kernel
        make build arch=riscv32 init=$(cat ../tests/$f)
        exec timeout 10s make justruntest arch=riscv32 init=$(cat ../tests/$f)
    ) &

    pid=$!

    wait $pid

    diff -I 'bbl loader' -I 'Hello RISCV! in hart' -u ${f%.cmd}.out stdout || { echo 'testing failed for' $f; exit 1; }

    echo testing $f pass
done

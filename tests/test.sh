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

    awk 'NR > 25 { print }' < stdout > stdout.new

    diff -u ${f%.cmd}.out stdout.new || { echo 'testing failed for' $f; exit 1; }

    echo testing $f pass
done

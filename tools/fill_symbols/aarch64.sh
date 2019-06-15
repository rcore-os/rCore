#!/bin/bash
echo "Filling kernel symbols."
rcore=$1
tmpfile=$(mktemp /tmp/rcore-symbols.txt.XXXXXX)
echo "Writing symbol table."
$2nm $1 >$tmpfile
gzip $tmpfile
tmpfile=$tmpfile.gz
symbol_table_loc=$((16#$($2objdump -D $rcore -j .data -F |grep "<rcore_symbol_table>" |grep -oEi "0x[0-9a-f]+" |grep -oEi "[0-9a-f][0-9a-f]+")))
symbol_table_size_loc=$((16#$($2objdump -D $rcore -j .data -F |grep "<rcore_symbol_table_size>" |grep -oEi "0x[0-9a-f]+" |grep -oEi "[0-9a-f][0-9a-f]+")))
echo $symbol_table_loc
echo $symbol_table_size_loc
FILESIZE=$(stat -c%s "$tmpfile")
echo $FILESIZE
dd bs=4096 count=$FILESIZE if=$tmpfile of=$rcore seek=$symbol_table_loc conv=notrunc iflag=count_bytes oflag=seek_bytes
echo "Writing size"
python3 -c "open('$tmpfile', 'wb').write(($FILESIZE).to_bytes(8,'little'))"
FILESIZE=$(stat -c%s "$tmpfile")
echo $FILESIZE
dd bs=1 count=$FILESIZE if=$tmpfile of=$rcore seek=$symbol_table_size_loc conv=notrunc
rm $tmpfile
echo "Done."

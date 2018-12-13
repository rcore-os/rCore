#!/bin/bash

# description: shortcut for make project
#
# The changes of shortcut will be ignored by git.
# To directly remove it from repository, you can use:
#   `git rm --cached shortcut.sh`

args="arch=aarch64 board=raspi3 prefix=aarch64-linux-gnu-"

echo $0 $1 $2 $3 $4 $5 $6 $7 $8 $9

if [ "$1" == "build" ] ; then
	cd kernel
	echo +make build $args
	make build $args
fi

if [ "$1" == "justrun" ] ; then
	cd kernel
	echo +make justrun $args
	make justrun $args
fi

if [ "$1" == "run" ] ; then
	cd kernel
	echo +make run $args
	make run $args
fi

if [ "$1" == "clean" ] ; then
	cd kernel
	echo +make clean
	make clean
fi


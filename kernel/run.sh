#!/bin/bash
sleep 10 && sudo ifconfig tap0 10.0.0.1 & 
make run net=on arch=x86_64 mode=release

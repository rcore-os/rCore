.PHONY: all run debug clean user

all:
	cd user && make sfsimg
	cd kernel && make all

release: 
	cd user && make sfsimg arch=x86_64 MODE=release
	cd kernel && make all ARCH=x86_64 MODE=release && make run ARCH=x86_64

log: 
	cd user && make sfsimg arch=x86_64 MODE=release
	cd kernel && make all ARCH=x86_64 MODE=release && make run ARCH=x86_64 LOG=debug

debug:
	cd user && make sfsimg arch=x86_64 MODE=debug
	cd kernel && make all ARCH=x86_64 MODE=debug && make clion_debug ARCH=x86_64 MODE=debug LOG=debug

clean:
	cd kernel && make clean
	cd user && make clean

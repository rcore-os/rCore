.PHONY: all run debug clean

all:
	cd user && make sfsimg arch=x86_64
run: all
	cd kernel && make run ARCH=x86_64 mode=release
debug: all
	cd kernel && make run ARCH=x86_64 LOG=info
clean:
	cd kernel && make clean
	cd user && make clean

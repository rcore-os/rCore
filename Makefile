.PHONY: all run debug clean user

release: 
	cd user && make sfsimg arch=x86_64 MODE=release
	cd kernel && make all ARCH=x86_64 MODE=release

debug:
	cd user && make sfsimg arch=x86_64 MODE=debug
	cd kernel && make all ARCH=x86_64 MODE=debug && make debug ARCH=x86_64 MODE=debug

clean:
	cd kernel && make clean
	cd user && make clean

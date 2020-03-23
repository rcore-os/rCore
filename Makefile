all:
	cd user && \
	make all arch=x86_64 && \
	make alpine arch=x86_64 && \
	make test arch=x86_64 && \
	make sfsimg arch=x86_64
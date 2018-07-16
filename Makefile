# used by docker_* targets
docker_image ?= blog_os
tag ?= 0.1
pwd ?= $(realpath ./)
ifeq ($(OS),Windows_NT)
uid ?= 0
gid ?= 0
innerpwd ?= /root/blog_os
else
uid ?= $(shell id -u)
gid ?= $(shell id -g)
innerpwd ?= $(pwd)
endif
docker_cargo_volume ?=  blogos-$(uid)-$(gid)-cargo
docker_rustup_volume ?=  blogos-$(uid)-$(gid)-rustup
docker_args ?= -e LOCAL_UID=$(uid) -e LOCAL_GID=$(gid) -v $(docker_cargo_volume):/usr/local/cargo -v $(docker_rustup_volume):/usr/local/rustup -v $(pwd):$(innerpwd) -w $(innerpwd)
docker_clean_args ?= $(docker_cargo_volume) $(docker_rustup_volume)

# docker_* targets

docker_build:
	@docker build docker/ -t $(docker_image):$(tag)

docker_iso:
	@docker run --rm $(docker_args) $(docker_image):$(tag) make iso

docker_interactive:
	@docker run -it --rm $(docker_args) $(docker_image):$(tag)

docker_clean:
	@docker volume rm $(docker_clean_args)

docker_riscv:
	@docker run -it --rm $(docker_args) wangrunji0408/riscv-rust
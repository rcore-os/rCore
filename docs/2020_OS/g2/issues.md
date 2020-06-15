# 问题记录

## 找不到 xbuild

- 报错

```
error: no such subcommand: `xbuild`

        Did you mean `build`?
Makefile:41: recipe for target 'rust' failed
```

- 解决

```bash
cargo install cargo-xbuild
```

## 找不到 autoreconf

- 报错

```
/bin/sh: 1: autoreconf: not found
```

- 解决

```bash
sudo apt install autoconf
```

- 再报错

```
cd build/x86_64/iperf-3.6 && autoreconf
configure.ac:48: error: possibly undefined macro: AC_PROG_LIBTOOL
      If this token and others are legitimate, please use m4_pattern_allow.
      See the Autoconf documentation.
```

- 再解决

[https://github.com/maxmind/libmaxminddb/issues/9](ref)

```bash
sudo apt install libtool
```

## 

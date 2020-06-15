## libc-test 

### 安装

直接使用我们的 `user` 子模块即可。如果要自己设置，在 `user` 目录下执行

```bash
$ git clone git://repo.or.cz/libc-test
$ rm -r libc-test/.git
```

### 编译

考虑到在 rCore 中编译所有测例耗费时间过长，所以选择在本机用 musl-gcc 编译。请确保自己已经安装好了`x86_64-linux-musl-gcc` 工具链，且在 path 中，在 `config.make` 制定了 `CC` 的值。 
在本机执行：

```bash
$ make
$ rm src/*/*.err
```

随后修改 `user` 目录下的 `Makefile` 文件，将 `libc-test`打包进入文件系统。

### 在 rCore 中测试

进入 `libc-test` 目录，执行脚本

```bash
$ ash runtest.sh
```

在测试测例前控制台会先打印当前测例名。若测试成功则顺次测试下一个测例，若失败则会打印额外信息，当遇到更严重的错误时可能导致 rCore 卡死或崩溃。例如在测试 `math` 库中的 `sqrt` 时，若测试失败，则输出为

```
run sqrt
sqrt failed
```

在结束后，可前往对应测例所在目录下，通过查看测例所对应的 `.err` 文件查看失败的原因。

当遇到使得 rCore 崩溃的测例时，手动记录当前测例在 `runtest.sh` 中的位置，手动更新 `user` 中的文件，使其从下一个测例开始测试，并记录中间若干测例的测试结果。如此反复，直到测试过所有测例。

### 目前测试的结果

- [ ] 尚未通过的测例 (215/473)
    - [ ] `pthread` 相关：可能由于缺少相关信号
    - [ ] `math` 相关：由于缺少对 `mxcsr` 寄存器的支持，导致获取 `FP Exceptions` 失败，从而无法通过相关测例中对 `FP Exceptions` 的校验。极少数情况出现对于 bad cases 的计算错误。
    - [ ] `sync` 相关

具体的测试结果可参考 `user/libc-test/` 目录下的三个 `RECORD.txt` 文件。

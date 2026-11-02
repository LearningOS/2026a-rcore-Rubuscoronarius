# 实验四：文件系统与 I/O

## 实现概要

实现 linkat、unlinkat 和 fstat：目录项可指向同一 inode，磁盘 inode 保存链接数；删除目录项后，最后一个打开的文件对象关闭时回收 inode 和数据块。目录空槽可复用，文件状态返回 inode 编号、类型和链接数。迁移前一章的内存映射、时间、spawn 和 stride 调度功能，并让 spawn 从文件系统读取可执行文件。

## 第六章问答

**root inode 的作用及损坏的影响。** easy-fs 将 inode 0 作为根目录；它的数据块保存文件名到 inode 编号的目录项，路径查找从这里开始。根 inode 的类型、大小或块指针损坏可能使根目录无法遍历，导致文件无法通过名称访问、读取错误甚至内核崩溃；即便其他 inode 和数据块仍在磁盘上，也需要借助修复工具重建目录关系才能恢复访问。

## 第七章问答

1. 在 shell 中执行 `cat file.txt | wc -l`，管道把 cat 的标准输出连接到 wc 的标准输入，从而统计文件行数。
2. 可设计内核管理的消息队列：进程通过名称或句柄连接同一个队列，发送时附目的进程或主题，接收时按目标或主题取消息。内核负责缓冲、阻塞与唤醒，并规定权限、队列容量和关闭行为。这样不必为每一对进程单独维护管道；广播、消息顺序和满队列处理仍需明确约定。

## 验证结果

```
python3 check/ch6.py < stdout-ch6 || (\
	make restore ; \
	exit 1 ; \
)
['Test power_3 OK2200!', 'Test fstat OK2200!', 'Test spawn0 OK2200!', 'Test write A OK2200!', 'get_time OK2200! (\\d+)', 'Test sleep OK2200!', 'Test 04_5 ummap OK2200!', 'current time_msec = (\\d+)', 'Test sleep1 passed2200!', 'Hello, world from user mode program!', 'hello child process!', 'forktest pass.', 'Test file0 OK2200!', 'child process pid = (\\d+), exit code = (\\d+)', 'Test 04_1 OK2200!', 'Test wait OK2200!', 'Test link OK2200!', 'Test waitpid OK2200!', 'Test write B OK2200!', 'file_test passed2200!', 'Test 04_6 ummap2 OK2200!', 'Test sbrk almost OK2200!', 'Test write C OK2200!', 'time_msec = (\\d+) after sleeping (\\d+) ticks, delta = (\\d+)ms!', 'Test power_5 OK2200!', 'Test power_7 OK2200!', 'exit pass.', 'Test mass open/unlink OK2200!', 'Test 04_4 test OK2200!'] ['FAIL: T.T', 'Test sbrk failed!']
[PASS] found <Test power_3 OK2200!>
[PASS] found <Test fstat OK2200!>
[PASS] found <Test spawn0 OK2200!>
[PASS] found <Test write A OK2200!>
[PASS] found <get_time OK2200! (\d+)>
[PASS] found <Test sleep OK2200!>
[PASS] found <Test 04_5 ummap OK2200!>
[PASS] found <current time_msec = (\d+)>
[PASS] found <Test sleep1 passed2200!>
[PASS] found <Hello, world from user mode program!>
[PASS] found <hello child process!>
[PASS] found <forktest pass.>
[PASS] found <Test file0 OK2200!>
[PASS] found <child process pid = (\d+), exit code = (\d+)>
[PASS] found <Test 04_1 OK2200!>
[PASS] found <Test wait OK2200!>
[PASS] found <Test link OK2200!>
[PASS] found <Test waitpid OK2200!>
[PASS] found <Test write B OK2200!>
[PASS] found <file_test passed2200!>
[PASS] found <Test 04_6 ummap2 OK2200!>
[PASS] found <Test sbrk almost OK2200!>
[PASS] found <Test write C OK2200!>
[PASS] found <time_msec = (\d+) after sleeping (\d+) ticks, delta = (\d+)ms!>
[PASS] found <Test power_5 OK2200!>
[PASS] found <Test power_7 OK2200!>
[PASS] found <exit pass.>
[PASS] found <Test mass open/unlink OK2200!>
[PASS] found <Test 04_4 test OK2200!>
[PASS] not found <FAIL: T.T>
[PASS] not found <Test sbrk failed!>

Test passed2200: 31/31
```

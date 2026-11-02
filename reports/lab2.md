# 实验二：地址空间

## 实验内容

为引入虚拟内存后的内核实现 `get_time`、`trace`、`mmap` 和 `munmap`。`get_time` 通过页表翻译用户缓冲区，按页复制时间结构，处理跨页情况；`trace` 查询当前任务的调用计数，读取或写入时检查页表有效位、用户权限和读写权限。

`mmap` 检查起始地址对齐、范围溢出和权限，先检查所有目标页是否空闲，再逐页分配并映射物理页。`munmap` 先确认范围内的页均存在且有对应物理页，再逐页撤销映射并回收帧。系统调用计数保存在各任务控制块中，在系统调用分发时更新。

## 地址翻译和页表

Sv39 虚拟地址分为三级页索引和 12 位页内偏移。处理器从 satp 指向的根页表逐级查找页表项，检查有效位和访问权限后，将叶子页表项中的物理页号与页内偏移组合成物理地址。缺页或权限不满足时产生异常，操作系统负责处理异常和维护映射。

页表项包含物理页号和 V、R、W、X、U、G、A、D 等标志：V 表示有效，R/W/X 表示读写执行权限，U 控制用户态访问，G 标记全局映射，A/D 分别表示访问和写入状态。TLB 缓存地址翻译结果，修改映射后需要确保旧翻译不会被继续使用。

## 运行结果

```
python3 check/ch4.py < stdout-ch4 || (\
	make restore ; \
	exit 1 ; \
)
['get_time OK27930! (\\d+)', 'Test sleep OK27930!', 'current time_msec = (\\d+)', 'time_msec = (\\d+) after sleeping (\\d+) ticks, delta = (\\d+)ms!', 'Test sleep1 passed27930!', 'string from task trace test', 'Test trace OK27930!', 'Test 04_1 OK27930!', 'Test 04_4 test OK27930!', 'Test 04_5 ummap OK27930!', 'Test 04_6 ummap2 OK27930!', 'Test trace_1 OK27930!'] ['FAIL: T.T', 'Test sbrk failed!', 'Should cause error, Test 04_2 fail!', 'Should cause error, Test 04_3 fail!']
[PASS] found <get_time OK27930! (\d+)>
[PASS] found <Test sleep OK27930!>
[PASS] found <current time_msec = (\d+)>
[PASS] found <time_msec = (\d+) after sleeping (\d+) ticks, delta = (\d+)ms!>
[PASS] found <Test sleep1 passed27930!>
[PASS] found <string from task trace test>
[PASS] found <Test trace OK27930!>
[PASS] found <Test 04_1 OK27930!>
[PASS] found <Test 04_4 test OK27930!>
[PASS] found <Test 04_5 ummap OK27930!>
[PASS] found <Test 04_6 ummap2 OK27930!>
[PASS] found <Test trace_1 OK27930!>
[PASS] not found <FAIL: T.T>
[PASS] not found <Test sbrk failed!>
[PASS] not found <Should cause error, Test 04_2 fail!>
[PASS] not found <Should cause error, Test 04_3 fail!>

Test passed27930: 16/16

```

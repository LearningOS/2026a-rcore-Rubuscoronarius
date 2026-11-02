# 实验三：进程管理与调度

## 实现概要

迁移 get_time、mmap 和 munmap，使其访问当前进程的地址空间；解除部分映射时拆分内存区域，以便后续 fork 正确复制。spawn 直接从目标 ELF 创建子进程，建立父子关系并加入就绪队列。调度器选择 stride 最小的就绪进程，按优先级累加步长；set_priority 校验并更新优先级。

## stride 溢出分析

1. 不会。8 位无符号整数回绕后，p2 的 stride 从 250 变为 4。若直接按无符号数比较，4 小于 255，调度器仍会选择 p2，尽管理论上 p1 的 255 小于 p2 的 260。
2. 初始 stride 相同。每次只让最小值增加一个 pass，其他进程的值不变；最大差距因此不会超过最大的单次 pass。优先级至少为 2，所以 pass 至多为 BigStride / 2。这个结论依赖这些初始条件以及严格选择最小 stride。
3. 在两个真实 stride 的差小于半个整数环的前提下，回绕差转换为有符号数可判断先后。BinaryHeap 是大顶堆，故比较结果需要反转：

```rust
fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some((self.0.wrapping_sub(other.0) as i64).cmp(&0).reverse())
}
```

该比较只适用于差值小于半个整数环的情形，不是任意整数的全序。本实现使用较宽的 u128 stride，避免实验期间发生回绕。

## 验证情况

```
python3 check/ch5.py < stdout-ch5 || (\
	make restore ; \
	exit 1 ; \
)
['Test sleep OK26726!', 'current time_msec = (\\d+)', 'time_msec = (\\d+) after sleeping (\\d+) ticks, delta = (\\d+)ms!', 'Test 04_6 ummap2 OK26726!', 'Test waitpid OK26726!', 'Test 04_5 ummap OK26726!', 'Test spawn0 OK26726!', 'Test set_priority OK26726!', 'get_time OK26726! (\\d+)', 'Test 04_4 test OK26726!', 'Test 04_1 OK26726!', 'Test sleep1 passed26726!', 'Test wait OK26726!'] ['FAIL: T.T', 'Test sbrk failed!']
[PASS] found <Test sleep OK26726!>
[PASS] found <current time_msec = (\d+)>
[PASS] found <time_msec = (\d+) after sleeping (\d+) ticks, delta = (\d+)ms!>
[PASS] found <Test 04_6 ummap2 OK26726!>
[PASS] found <Test waitpid OK26726!>
[PASS] found <Test 04_5 ummap OK26726!>
[PASS] found <Test spawn0 OK26726!>
[PASS] found <Test set_priority OK26726!>
[PASS] found <get_time OK26726! (\d+)>
[PASS] found <Test 04_4 test OK26726!>
[PASS] found <Test 04_1 OK26726!>
[PASS] found <Test sleep1 passed26726!>
[PASS] found <Test wait OK26726!>
[PASS] not found <FAIL: T.T>
[PASS] not found <Test sbrk failed!>

Test passed26726: 15/15
```

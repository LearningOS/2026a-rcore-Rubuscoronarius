# 实验五：并发与死锁检测

## 实现概要

为每个进程维护互斥锁和信号量的可用资源、线程持有量及等待请求，分别运行安全性检查；开启检测后拒绝不安全的获取请求并返回 -0xDEAD。实现检测开关和 get_time，保留原有线程及同步机制。

实验用时：待本人填写。

## 问答

### 主线程退出时的资源回收

进程需释放所有线程的用户栈、陷入上下文、线程 ID 和内核栈，以及进程地址空间、文件描述符和同步对象等资源；子进程需要重新托管，进程控制块保留为僵尸状态直到父进程回收。线程控制块可能被进程的 tasks 表、调度器就绪队列、定时器队列，以及互斥锁、信号量或条件变量的等待队列引用。退出时要从全局就绪队列和定时器中移除，否则线程仍可能被调度或被计时器唤醒；进程私有同步对象的等待队列随对象释放而消失。当前线程的内核栈不能在其上执行时释放，因此需要延迟回收。

### 两种互斥锁的区别

Mutex1 解锁时先将锁置为空闲并唤醒等待者；被唤醒的线程必须循环重新竞争。新到的线程可能先抢到锁，使等待者再次睡眠，可能饥饿，但循环保证最终持锁者才能进入临界区。Mutex2 在存在等待者时保持锁定并将锁直接交给被唤醒者；它醒来后不再竞争，减少抢占和重复等待，但依赖唤醒只发生一次且该线程确实获得锁；若线程被错误唤醒或接收锁的线程退出，可能破坏互斥或使锁永久占用。

## 验证结果

```
python3 check/ch8.py < stdout-ch8 || (\
	make restore ; \
	exit 1 ; \
)
['Test write C OK23908!', 'hello child process!', 'pipetest passed23908!', 'Test power_7 OK23908!', 'deadlock test semaphore 1 OK23908!', 'sync_sem passed23908!', 'exit pass.', 'threads with arg test passed23908!', 'deadlock test semaphore 2 OK23908!', 'Test power_3 OK23908!', 'Test write A OK23908!', 'mpsc_sem passed23908!', 'file_test passed23908!', 'threads test passed23908!', 'test_condvar passed23908!', 'race adder using spin mutex test passed23908!', 'forktest pass.', 'deadlock test mutex 1 OK23908!', 'Test write B OK23908!', 'Test power_5 OK23908!', 'child process pid = (\\d+), exit code = (\\d+)', 'Hello, world from user mode program!', 'philosopher dining problem with mutex test passed23908!'] ['FAIL: T.T', 'Test sbrk failed!']
[PASS] found <Test write C OK23908!>
[PASS] found <hello child process!>
[PASS] found <pipetest passed23908!>
[PASS] found <Test power_7 OK23908!>
[PASS] found <deadlock test semaphore 1 OK23908!>
[PASS] found <sync_sem passed23908!>
[PASS] found <exit pass.>
[PASS] found <threads with arg test passed23908!>
[PASS] found <deadlock test semaphore 2 OK23908!>
[PASS] found <Test power_3 OK23908!>
[PASS] found <Test write A OK23908!>
[PASS] found <mpsc_sem passed23908!>
[PASS] found <file_test passed23908!>
[PASS] found <threads test passed23908!>
[PASS] found <test_condvar passed23908!>
[PASS] found <race adder using spin mutex test passed23908!>
[PASS] found <forktest pass.>
[PASS] found <deadlock test mutex 1 OK23908!>
[PASS] found <Test write B OK23908!>
[PASS] found <Test power_5 OK23908!>
[PASS] found <child process pid = (\d+), exit code = (\d+)>
[PASS] found <Hello, world from user mode program!>
[PASS] found <philosopher dining problem with mutex test passed23908!>
[PASS] not found <FAIL: T.T>
[PASS] not found <Test sbrk failed!>

Test passed23908: 25/25
```

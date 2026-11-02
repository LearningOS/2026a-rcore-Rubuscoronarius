# lab1 实验报告：系统调用追踪

## 1 实验目标

第 3 章的内核已经具备多道程序装入与分时调度的能力，但缺少观察用户程序系统调用行为的接口。本实验在该章内核中增加编号为 410 的 trace 系统调用，要求它按任务记录每个系统调用被调用的次数并允许用户程序查询，读取用户地址空间中的一个字节，以及向用户地址空间写入一个字节。

统计功能用于观察用户程序实际发起了哪些系统调用，读写功能用于检验内核依据用户给出的地址访问内存时是否正确。三项要求由 rCore-Tutorial-Test 仓库中的 ch3_trace 程序统一验收。

## 2 接口语义

### 2.1 请求编号与返回值

trace 调用的三个参数依次表示请求类型、地址或系统调用号、待写入的数据，具体含义取决于用户库的定义。用户库中的枚举 TraceRequest 按 Read、Write、Syscall 的顺序声明成员，Rust 依据声明顺序赋予判别值，因此这三种请求在系统调用入口处的编号依次为 0、1、2。

阅读测试程序时需要注意这一点。公开资料中常见把 Syscall 放在首位的写法，与当前仓库的代码不一致，按那种写法实现会使全部断言失败。

返回值的约定同样由用户库的封装决定。trace_read 把读到的字节作为结果返回，内核给出 -1 时它转换为 None；trace_write 只在成功时返回 0；count_syscall 直接返回计数值，用户库不对它做额外处理。

### 2.2 测试对实现提出的约束

ch3_trace 在第二次 trace 调用之后立即查询 SYSCALL_TRACE 的计数，要求结果为 2。程序输出一行字符串，再经过几次查询后，同一接口要求返回 7。这两个数字都比不含本次调用的结果大 1，说明计数必须在请求被分发之前完成。字符串输出前后的 write 计数由 0 变为 2，该断言同时表明计数值按任务独立保存，其他任务的输出不会累加进来。查询 yield 的断言只要求结果大于 0，因为用户库的 sleep 通过反复查询时间并在到期之前让出处理器来实现等待，500 毫秒的等待会产生大量 yield 调用。读写能力由两次访问验证，程序先读取栈上的变量 var 并确认其值为 111，再向同一地址写入新值后由用户程序读回比对，随后它读取 main 函数的地址，确认合法代码段地址不会被判为错误。

### 2.3 write 计数为 2 的原因

测试程序里有一句注释，问的是 write 为何被调用两次。用户库的 println 用 concat! 在格式串末尾追加了一个换行符，而控制台缓冲区的实现每遇到换行符就调用一次 write 把缓冲区内容送出。测试写作 println!("string from task trace test\n")，实际送出的字节序列含有两个换行符，于是产生两次 write 调用。

## 3 设计

### 3.1 计数表的存放位置

第 3 章的任务控制块需要实现 Copy 才能用数组字面量初始化全部任务，把计数表作为它的字段时，这个数组也必须保持 Copy，每次按值传递都会复制整块数据。更关键的限制来自栈空间。os/src/entry.asm 中启动栈由 `.space 4096 * 16` 分配，容量为 64KB。计数表的规模取 16 个任务与 500 个系统调用号的乘积，按 u32 计算需要 32KB。TASK_MANAGER 是 lazy_static，首次访问时在 rust_main 的栈帧内构造，无论把表放进任务控制块还是 TaskManagerInner，构造过程中都可能同时存在局部变量与结构体临时值两份数据，合计超过启动栈的容量。裸机上的栈溢出表现为难以定位的异常，因此这两种位置都不采用。

最终的做法是在 .bss 中定义一个二维静态数组，两个下标分别对应任务号与系统调用号。静态数组由零初始化，构造过程不经过栈，占用空间与数据结构的组织方式无关。代价是访问需要 unsafe，本章内核运行在单核处理器上，计数只发生在系统调用路径上，不存在并发访问，因此用 allow(static_mut_refs) 标注这一前提。

系统调用号的上界取 500。trace 自身的编号是 410，本章其余系统调用号都小于该值，留出的余量足以覆盖后续章节，同时对越界编号做了检查，避免数组下标越界。

### 3.2 计数的位置

自增发生在 syscall 函数的开头，位于按编号分发的 match 之前。这个位置满足 2.2 节由断言确定的时序要求，也使所有系统调用被统一记录，无需在各自的处理函数中重复添加代码。

读取计数的函数先取得当前任务号，再访问数组。取得任务号的语句把 RefMut 临时值限制在单条语句之内，只复制出 usize 类型的下标，否则在 sys_trace 内部再次借用 TASK_MANAGER 时，RefCell 的运行时检查会失败。

### 3.3 用户地址的访问方式

第 3 章尚未启用分页，satp 寄存器没有设置，用户程序与内核运行在同一物理地址空间，用户传来的地址可以直接作为内核的指针使用。访问通过 read_volatile 与 write_volatile 完成，避免编译器把这类访问视为无用而删除，测试正是依赖写入后能被读回这一行为。

这种实现无法识别非法地址。若用户给出未映射的地址，内核会触发页错误，而不会返回 -1。第 3 章的用户程序无法构造这类地址，测试也只使用栈变量与代码段地址，因此当前版本可以满足要求。第 4 章引入地址空间之后，同一接口需要改为按页表逐页翻译，并对跨越页面的访问单独处理。

## 4 实现

### 4.1 新增常量

在 os/src/config.rs 中给出计数表的第二维长度。

```rust
/// the max number of syscall ids we trace
/// (sys_trace is 410, so 500 leaves enough room)
pub const MAX_SYSCALL_NUM: usize = 500;
```

### 4.2 任务模块

在 os/src/task/mod.rs 中声明计数表，并在 TaskManager 上增加访问它的两个方法。

```rust
use crate::config::{MAX_APP_NUM, MAX_SYSCALL_NUM};

static mut SYSCALL_TIMES: [[u32; MAX_SYSCALL_NUM]; MAX_APP_NUM] =
    [[0; MAX_SYSCALL_NUM]; MAX_APP_NUM];
```

```rust
    /// Count one more call of `syscall_id` for the current task.
    #[allow(static_mut_refs)]
    fn inc_syscall_count(&self, syscall_id: usize) {
        if syscall_id >= MAX_SYSCALL_NUM {
            return;
        }
        let current = self.inner.exclusive_access().current_task;
        unsafe {
            SYSCALL_TIMES[current][syscall_id] += 1;
        }
    }

    /// How many times the current task has called `syscall_id`.
    #[allow(static_mut_refs)]
    fn get_syscall_count(&self, syscall_id: usize) -> u32 {
        if syscall_id >= MAX_SYSCALL_NUM {
            return 0;
        }
        let current = self.inner.exclusive_access().current_task;
        unsafe { SYSCALL_TIMES[current][syscall_id] }
    }
```

模块级再提供两个包装函数，写法与该文件中已有的 run_first_task、suspend_current_and_run_next 保持一致。

```rust
pub fn inc_syscall_count(syscall_id: usize) { TASK_MANAGER.inc_syscall_count(syscall_id); }
pub fn get_syscall_count(syscall_id: usize) -> u32 { TASK_MANAGER.get_syscall_count(syscall_id) }
```

### 4.3 系统调用入口

在 os/src/syscall/mod.rs 的 syscall 函数开头完成计数，随后才进入按编号分发的分支。

```rust
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    // Count before dispatching, so that a `sys_trace` query can see the very
    // call that is asking (that is what the checker expects).
    crate::task::inc_syscall_count(syscall_id);
    match syscall_id {
        ...
        SYSCALL_TRACE => sys_trace(args[0], args[1], args[2]),
        ...
    }
}
```

### 4.4 trace 的实现

os/src/syscall/process.rs 中原有的空实现替换为按请求类型分派的三条分支。

```rust
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        // TraceRequest::Read
        0 => unsafe { (id as *const u8).read_volatile() as isize },
        // TraceRequest::Write
        1 => {
            unsafe { (id as *mut u8).write_volatile(data as u8) };
            0
        }
        // TraceRequest::Syscall
        2 => crate::task::get_syscall_count(id) as isize,
        _ => -1,
    }
}
```

读取分支把读到的字节作为返回值，写入分支在完成后返回 0，查询分支返回计数值，无法识别的请求类型返回 -1。写入时只取 data 的最低 8 位，与用户库中 trace_write 的参数类型一致。

## 5 编译与运行

### 5.1 编译

```bash
make -C os build
```

该命令先构建用户程序，再把内核与用户程序一并链接，产物位于 os/target/riscv64gc-unknown-none-elf/release/os.bin。

### 5.2 装入测试程序

在仓库根目录执行 make -C os run 时，os/Makefile 把 BASE 的默认值 1 传给用户程序的构建脚本。user/Makefile 依据 BASE 与 TEST 挑选源文件，BASE 为 1 时只选取文件名形如 chNb_ 的基础演示程序，ch3_trace 不在其中。把 BASE 设为 2 之后，第 2 章与第 3 章的全部用户程序都会参与构建并被嵌入内核。

```bash
make -C os run BASE=2
```

### 5.3 运行结果

```
[kernel] Hello, world!
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
Hello, world from user mode program!
power_5 [10000/140000]
power_5 [20000/140000]
power_5 [30000/140000]
power_5 [40000/140000]
power_5 [50000/140000]
power_5 [60000/140000]
power_5 [70000/140000]
power_5 [80000/140000]
power_5 [90000/140000]
power_7 [10000/160000]
power_7 [20000/160000]
power_7 [30000/160000]
power_7 [40000/160000]
power_7 [50000/160000]
power_7 [60000/160000]
power_7 [70000/160000]
power_7 [80000/160000]
power_7 [90000/160000]
power_7 [100000/160000]
power_7 [110000/160000]
power_7 [120000/160000]
power_7 [130000/160000]
power_7 [140000/160000]
get_time OK! 96
current time_msec = 98
AAAAAAAAAA [1/5]
BBBBBBBBBB [1/5]
CCCCCCCCCC [1/5]
power_3 [10000/200000]
power_3 [20000/200000]
power_3 [30000/200000]
power_5 [100000/140000]
power_5 [110000/140000]
power_5 [120000/140000]
power_5 [130000/140000]
power_5 [140000/140000]
5^140000 = 386471875(MOD 998244353)
Test power_5 OK!
power_7 [150000/160000]
power_7 [160000/160000]
7^160000 = 667897727(MOD 998244353)
Test power_7 OK!
AAAAAAAAAA [2/5]
BBBBBBBBBB [2/5]
CCCCCCCCCC [2/5]
power_3 [40000/200000]
power_3 [50000/200000]
power_3 [60000/200000]
power_3 [70000/200000]
power_3 [80000/200000]
power_3 [90000/200000]
power_3 [100000/200000]
power_3 [110000/200000]
power_3 [120000/200000]
power_3 [130000/200000]
power_3 [140000/200000]
power_3 [150000/200000]
AAAAAAAAAA [3/5]
CCCCCCCCCC [3/5]
power_3 [160000/200000]
power_3 [170000/200000]
power_3 [180000/200000]
power_3 [190000/200000]
power_3 [200000/200000]
3^200000 = 871008973(MOD 998244353)
Test power_3 OK!
AAAAAAAAAA [4/5]
BBBBBBBBBB [3/5]
CCCCCCCCCC [4/5]
AAAAAAAAAA [5/5]
BBBBBBBBBB [4/5]
CCCCCCCCCC [5/5]
Test write A OK!
BBBBBBBBBB [5/5]
Test write B OK!
Test write C OK!
time_msec = 200 after sleeping 100 ticks, delta = 102ms!
Test sleep1 passed!
string from task trace test

Test trace OK!
Test sleep OK!

```

三个测试程序的输出与预期一致。ch3_trace 中共有 13 次 trace 调用，覆盖两次计数查询、五次其他系统调用查询以及三次内存访问，全部断言成立，说明计数时机、任务归属与用户内存访问三个方面都符合要求。


## 6 小结

本次实验为第 3 章内核补充了系统调用统计与用户内存访问的能力，改动集中在四个文件中。

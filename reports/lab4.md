# 实验四：文件系统与 I/O

## 实现概要

实现 linkat、unlinkat 和 fstat：目录项可指向同一 inode，磁盘 inode 保存链接数；删除目录项后，最后一个打开的文件对象关闭时回收 inode 和数据块。目录空槽可复用，文件状态返回 inode 编号、类型和链接数。迁移前一章的内存映射、时间、spawn 和 stride 调度功能，并让 spawn 从文件系统读取可执行文件。

## 第六章问答

**root inode 的作用及损坏的影响。** easy-fs 将 inode 0 作为根目录；它的数据块保存文件名到 inode 编号的目录项，路径查找从这里开始。根 inode 的类型、大小或块指针损坏可能使根目录无法遍历，导致文件无法通过名称访问、读取错误甚至内核崩溃；即便其他 inode 和数据块仍在磁盘上，也需要借助修复工具重建目录关系才能恢复访问。

## 第七章问答

1. 在 shell 中执行 `cat file.txt | wc -l`，管道把 cat 的标准输出连接到 wc 的标准输入，从而统计文件行数。
2. 可设计内核管理的消息队列：进程通过名称或句柄连接同一个队列，发送时附目的进程或主题，接收时按目标或主题取消息。内核负责缓冲、阻塞与唤醒，并规定权限、队列容量和关闭行为。这样不必为每一对进程单独维护管道；广播、消息顺序和满队列处理仍需明确约定。

## 验证

- `cargo check --offline`（在 os 目录，以课程配置的 RISC-V 目标执行）：通过。
- `cargo test --manifest-path easy-fs/Cargo.toml --offline --target x86_64-pc-windows-msvc`：通过。测试覆盖硬链接、链接数、删除后已打开文件可读、inode 重用和间接数据块回收。
- 官方 QEMU 测试尚未运行：当前机器未检测到 QEMU、make 或可用的 Linux/WSL 发行版，也没有官方测试程序和检查器。

## 荣誉准则与资料说明

本次实现及报告由 Codex 协助生成和修改，并参考了 [第六章练习](https://learningos.cn/rCore-Tutorial-Guide/chapter6/4exercise.html)、[第七章练习](https://learningos.cn/rCore-Tutorial-Guide/chapter7/3exercise.html)及仓库已有实现。请提交者按实际交流情况补充说明，并依据课程的[荣誉准则](https://learningos.cn/rCore-Tutorial-Guide/honorcode.html)亲自核实和填写独立完成、代码来源及其他声明；本报告不代替提交者作事实承诺。

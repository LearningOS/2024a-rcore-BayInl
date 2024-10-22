# 实现的功能
完成了获取任务信息的功能，可以查询当前正在执行的任务信息，任务信息包括任务控制块相关信息（任务状态）、任务使用的系统调用及调用次数、系统调用时刻距离任务第一次被调度时刻的时长（单位ms）

# 简答作业
## 1.
1. 使用cargo rustc --bin ch2b_bad_xxx命令编译后，再使用objcopy 工具将ELF文件删除所有 ELF header 和符号得到 .bin 后缀的纯二进制镜像文件
2. 使用qemu-riscv64命令运行这些二进制文件
   - ch2b_bad_address报错：“Panicked at library/core/src/panicking.rs:220, unsafe precondition(s) violated: ptr::write_volatile requires that the pointer argument is aligned and non-null”
   - ch2b_bad_instructions报错：185813 illegal hardware instruction (core dumped)  qemu-riscv64 ch2b_bad_instructions
   - ch2b_bad_register报错：186117 illegal hardware instruction (core dumped)  qemu-riscv64 ch2b_bad_register

使用的sbi版本为 v0.3.1

## 2.
> 深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:
### 2.1
> L40：刚进入 __restore 时，a0 代表了什么值。请指出 __restore 的两种使用情景。

a0寄存器用于存放函数调用的参数与返回值。经过__alltraps函数后，寄存器 a0 指向内核栈的栈指针也就是刚刚保存的 Trap 上下文的地址，而后调用了trap_handler函数。
trap_handler的参数cx来自于a0寄存器，所以这样trap_handler就获得了Trap 上下文的地址。
__restore的两种使用情景：
- 从中断处理函数返回
- 从系统调用返回

### 2.2
> L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。
> ```asm
> ld t0, 32*8(sp)
> ld t1, 33*8(sp)
> ld t2, 2*8(sp)
> csrw sstatus, t0
> csrw sepc, t1
> csrw sscratch, t2
> ```
特殊处理了sstatus, sepc, sscratch三个寄存器。
这些寄存器均是S 特权级中与 Trap 相关的 控制状态寄存器。
- sstatus寄存器：SPP 等字段给出 Trap 发生之前 CPU 处在哪个特权级（S/U）等信息
- sepc寄存器：保存了 Trap 发生时 PC 的值，即 Trap 发生时正在执行的指令地址
- sscratch寄存器：中转寄存器，在保存 Tap 上下文的时候，它起到了两个作用:首先是保存了内核栈的地址，其次它可作为一个中转站让 sp(目前指向的用户栈的地址)的值可以暂时保存在 sscratch。

### 2.3
> L50-L56：为何跳过了 x2 和 x4？
> ```asm
> ld x1, 1*8(sp)
> ld x3, 3*8(sp)
> .set n, 5
> .rept 27
>    LOAD_GP %n
>    .set n, n+1
> .endr
> ```

x2寄存器是sp寄存器，用于指向当前线程的内核栈栈顶。这里我们会用该寄存器来找到个寄存器应该被保存到的正确位置，因此跳过了该寄存器的加载。
x4寄存器是一个固定用途的寄存器，被称为tp（Thread Pointer），用于指向当前线程的控制块。这里我们并不会用到该寄存器，因此跳过了该寄存器。
### 2.4
> L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？

csrrw的原型是 csrrw rd, csr, rs，可以将 CSR 当前的值读到通用寄存器 rd 中，然后将通用寄存器 rs 的值写入该 CSR 。因此这里csrrw sp, sscratch, sp的意思是将sscratch的值写入sp，然后将sp的值写入sscratch。

由于sscratch在进入内核态时被__alltraps设置为用户栈栈顶指针，因此执行csrrw sp, sscratch, sp后，sp指向用户栈栈顶。
### 2.5 
> __restore：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

发生状态切换在sret指令，执行后会从内核态切换到用户态
因为在执行该指令之前，已还原了用户态执行任务的上下文（包括通用寄存器、临时寄存器等），并且用csrrw指令交换了sp和sscratch的值，sp 重新指向用户栈栈顶，sscratch指向内核栈栈顶。
### 2.6
> L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？
> ```asm
> csrrw sp, sscratch, sp
> ```


### 2.7
> 从 U 态进入 S 态是哪一条指令发生的？

从用户函数库中的syscall函数中core::arch::asm!开始，执行ecall汇编指令后，进入内核态，执行trap.S中的__alltraps函数，


# 荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

**无**

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

**无**（除教材外）

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
	.text
	globl go
go:
	pushq %rbx
	pushq %rbp
	pushq %r12
	pushq %r13
	pushq %r14
	pushq %r15
	call _main
	popq %r15
	popq %r14
	popq %r13
	popq %r12
	popq %rbp
	popq %rbx
	retq
_main
	subq $8
	movq %r12, 0(%rsp)
	movq $_initArray_return, -8(%rsp)
	subq $8, %rsp
	jmp _initArray
_initArray_return:
	movq %rax, r12
	movq %r12, rdi
	movq $_sortArray_return, -8(%rsp)
	subq $8, %rsp
	jmp _sortArray
_sortArray_return:
	movq %r12, rdi
	movq $_printArray_return, -8(%rsp)
	subq $8, %rsp
	jmp _printArray
_printArray_return:
	movq 0(%rsp), %r12
	addq $8, %rsp
	retq
_initArray
	subq $24
	movq %r12, 0(%rsp)
	movq %r13, 8(%rsp)
	movq %r14, 16(%rsp)
	call input
	movq %rax, r12
	movq %r12, rdi
	movq $1, rsi
	call allocate
	movq %rax, r14
	movq $1, r13
_initArray_loop:
	cmpq %r13, %r12
	je _initArray_exit
	call input
	movq %r13, r10
	sarq $1, %r10
	inc %r10
	imulq $8, %r10
	addq %r14, %r10
	movq %rax, 0(%r10)
	addq $2, %r13
	jmp _initArray_loop
_initArray_exit:
	movq %r14, rax
	movq 0(%rsp), %r12
	movq 8(%rsp), %r13
	movq 16(%rsp), %r14
	addq $24, %rsp
	retq
_sortArray
	movq 0(%rdi), %r10
	subq $1, %r10
	salq $1, %r10
	inc %r10
_sortArray_loop1:
	cmpq $0, %r10
	jle _sortArray_exit1
	movq $1, r11
_sortArray_loop2:
	cmpq %r10, %r11
	je _sortArray_exit2
	movq %r11, r9
	sarq $1, %r9
	inc %r9
	imulq $8, %r9
	movq %rdi, r8
	addq %r9, %r8
	movq 0(%r8), %r9
	movq 8(%r8), %rcx
	cmpq %r9, %rcx
	jle _else
	movq %rcx, 0(%r8)
	movq %r9, 8(%r8)
_else:
	addq $2, %r11
	jmp _sortArray_loop2
_sortArray_exit2:
	subq $2, %r10
	jmp _sortArray_loop1
_sortArray_exit1:
	retq
_printArray
	subq $16
	movq %r12, 0(%rsp)
	movq %r13, 8(%rsp)
	movq %rdi, r12
	movq $1, r13
_printArray_loop:
	movq 0(%r12), %r10
	salq $1, %r10
	inc %r10
	cmpq %r13, %r10
	je _printArray_exit
	movq %r13, r11
	sarq $1, %r11
	inc %r11
	imulq $8, %r11
	addq %r12, %r11
	movq 0(%r11), %rdi
	call print
	addq $2, %r13
	jmp _printArray_loop
_printArray_exit:
	movq 0(%rsp), %r12
	movq 8(%rsp), %r13
	addq $16, %rsp
	retq

.global fpe
fpe:
	movsd	.LC0(%rip), %xmm0
	subsd	%xmm0, %xmm0
	stmxcsr -4(%rsp)
	mov %eax, -4(%rsp)
	ret
.LC0:
	.long	0
	.long	2146435072
	.long	0
	.long	0

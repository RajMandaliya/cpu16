; ─────────────────────────────────────────────────────────────────────────────
; stack_calc.asm  —  RPN (Reverse Polish Notation) stack calculator.
;
; Demonstrates: PUSH, POP, CALL, RET, a software stack for operands,
;               dispatcher pattern (jump table via CMP chains),
;               and subroutines for each arithmetic operation.
;
; Evaluates the RPN expression:  3  4  +  2  *  7  -
;   Step-by-step:
;     push 3          stack: [3]
;     push 4          stack: [3, 4]
;     +               stack: [7]        (3+4)
;     push 2          stack: [7, 2]
;     *               stack: [14]       (7*2)
;     push 7          stack: [14, 7]
;     -               stack: [7]        (14-7)
;   Result = 7  →  expected R0 = 7 after HALT
;
; The operand stack lives in memory at 0x0400 and grows upward.
; A stack pointer for the operand stack is kept at memory address 0x0350.
;
; Register map:
;   R0 = top of stack value / return value / scratch
;   R1 = second operand / scratch
;   R2 = operand stack pointer (address of next free slot)
;   R3 = scratch
;
; Note: This uses a SOFTWARE operand stack (memory at 0x0400) separate
;       from the CPU hardware stack (0xFFFE). CALL/RET use the hardware
;       stack; operands use the software stack. This mirrors how real
;       stack machines (JVM, Forth, CPython) work.
;
; Cycle count: ~180 cycles
; ─────────────────────────────────────────────────────────────────────────────

; ── Initialise software stack pointer ────────────────────────────────────────

        LOAD  R2, 0x0400   ; operand stack base
        LOAD  R3, 0x0350
        STORE R2, R3       ; mem[0x0350] = 0x0400 (stack pointer)

; ── RPN program: 3 4 + 2 * 7 - ───────────────────────────────────────────────

        LOAD  R0, 3
        CALL  PUSH_OP      ; push 3

        LOAD  R0, 4
        CALL  PUSH_OP      ; push 4

        CALL  OP_ADD       ; pop 4, pop 3, push 7

        LOAD  R0, 2
        CALL  PUSH_OP      ; push 2

        CALL  OP_MUL       ; pop 2, pop 7, push 14

        LOAD  R0, 7
        CALL  PUSH_OP      ; push 7

        CALL  OP_SUB       ; pop 7, pop 14, push 7

        CALL  POP_OP       ; pop result into R0

        ; R0 = 7  (0x0007)
        HALT

; ─────────────────────────────────────────────────────────────────────────────
; PUSH_OP  —  Push R0 onto the software operand stack.
;
; in:  R0 = value to push
; clobbers: R2, R3
; ─────────────────────────────────────────────────────────────────────────────
PUSH_OP:
        LOAD  R3, 0x0350
        LOADM R2, R3       ; R2 = current stack pointer
        STORE R0, R2       ; mem[sp] = R0
        ADDI  R2, 2        ; sp += 2
        STORE R2, R3       ; save updated sp
        RET

; ─────────────────────────────────────────────────────────────────────────────
; POP_OP  —  Pop top of software operand stack into R0.
;
; out: R0 = popped value
; clobbers: R2, R3
; ─────────────────────────────────────────────────────────────────────────────
POP_OP:
        LOAD  R3, 0x0350
        LOADM R2, R3       ; R2 = current stack pointer
        ADDI  R2, -2       ; sp -= 2
        STORE R2, R3       ; save updated sp
        LOADM R0, R2       ; R0 = mem[sp]
        RET

; ─────────────────────────────────────────────────────────────────────────────
; OP_ADD  —  Pop two values, push their sum.
;
; stack: [..., a, b] → [..., a+b]
; ─────────────────────────────────────────────────────────────────────────────
OP_ADD:
        CALL  POP_OP       ; R0 = b (top)
        MOV   R1, R0       ; R1 = b
        CALL  POP_OP       ; R0 = a
        ADD   R0, R1       ; R0 = a + b
        CALL  PUSH_OP      ; push result
        RET

; ─────────────────────────────────────────────────────────────────────────────
; OP_SUB  —  Pop two values, push their difference (a - b).
;
; stack: [..., a, b] → [..., a-b]
; ─────────────────────────────────────────────────────────────────────────────
OP_SUB:
        CALL  POP_OP       ; R0 = b
        MOV   R1, R0       ; R1 = b
        CALL  POP_OP       ; R0 = a
        SUB   R0, R1       ; R0 = a - b
        CALL  PUSH_OP      ; push result
        RET

; ─────────────────────────────────────────────────────────────────────────────
; OP_MUL  —  Pop two values, push their product.
;
; stack: [..., a, b] → [..., a*b]
; ─────────────────────────────────────────────────────────────────────────────
OP_MUL:
        CALL  POP_OP       ; R0 = b
        MOV   R1, R0       ; R1 = b
        CALL  POP_OP       ; R0 = a
        MUL   R0, R1       ; R0 = a * b
        CALL  PUSH_OP      ; push result
        RET

; ─────────────────────────────────────────────────────────────────────────────
; OP_DIV  —  Pop two values, push their quotient (a / b).
;
; stack: [..., a, b] → [..., a/b]
; (included for completeness — not used in the demo expression above)
; ─────────────────────────────────────────────────────────────────────────────
OP_DIV:
        CALL  POP_OP       ; R0 = b
        MOV   R1, R0       ; R1 = b (divisor)
        CALL  POP_OP       ; R0 = a (dividend)
        DIV   R0, R1       ; R0 = a / b
        CALL  PUSH_OP      ; push result
        RET

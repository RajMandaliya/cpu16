; ─────────────────────────────────────────────────────────────────────────────
; factorial.asm  —  Compute 6! = 720 (0x02D0) using a CALL/RET subroutine.
;
; Demonstrates: CALL, RET, PUSH, POP, the stack.
;
; Calling convention:
;   Argument  in R0
;   Return    in R1
;   Caller saves R2, R3 if needed.
; ─────────────────────────────────────────────────────────────────────────────

        LOAD  R0, 6        ; compute 6!
        CALL  FACTORIAL
        ; R1 = 720 (0x02D0)
        HALT

; ── FACTORIAL subroutine ──────────────────────────────────────────────────────
; in:  R0 = n
; out: R1 = n!
FACTORIAL:
        LOAD  R1, 1        ; result = 1
        LOAD  R2, 0        ; zero register

FACT_LOOP:
        CMP   R0, R2       ; if n == 0, done
        JZ    FACT_DONE
        MUL   R1, R0       ; result *= n
        ADDI  R0, -1       ; n--
        JMP   FACT_LOOP

FACT_DONE:
        RET
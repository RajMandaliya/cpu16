; ─────────────────────────────────────────────────────────────────────────────
; fibonacci.asm  —  Compute Fibonacci(10) iteratively.
;
; Algorithm:
;   a = 0, b = 1
;   repeat 9 times:
;       tmp = a + b
;       a   = b
;       b   = tmp
;   result in R1  (expects 55 = 0x0037)
;
; Register map:
;   R0 = a  (previous term)
;   R1 = b  (current term)
;   R2 = loop counter (counts down from 9)
;   R3 = temp
; ─────────────────────────────────────────────────────────────────────────────

        LOAD  R0, 0        ; a = 0
        LOAD  R1, 1        ; b = 1
        LOAD  R2, 9        ; counter = 9 (we already have F(0)=0,F(1)=1, need 9 more steps)

LOOP:
        MOV   R3, R1       ; tmp = b
        ADD   R1, R0       ; b   = a + b
        MOV   R0, R3       ; a   = tmp (old b)
        ADDI  R2, -1       ; counter--

        LOAD  R3, 0        ; zero register for comparison
        CMP   R2, R3       ; Zero flag set when counter == 0
        JNZ   LOOP

DONE:
        ; R1 now holds Fibonacci(10) = 55 = 0x0037
        HALT
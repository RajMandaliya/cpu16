; ─────────────────────────────────────────────────────────────────────────────
; sieve.asm  —  Sieve of Eratosthenes: find all primes up to 30.
;
; Demonstrates: nested loops, memory as a boolean array, multiplication
;               for stride computation, indirect memory writes (STORE).
;
; Algorithm:
;   Initialise sieve[0..30] = 1 (all candidate)
;   sieve[0] = sieve[1] = 0    (0 and 1 are not prime)
;   for p = 2 to sqrt(30) ~ 5:
;       if sieve[p] == 1:
;           for m = p*p to 30 step p:
;               sieve[m] = 0
;
; Memory layout:
;   0x0300 + i*2  =  sieve[i]   (1 = prime candidate, 0 = composite)
;   i ranges 0..30 → 31 words → 62 bytes → 0x0300..0x033C
;
; Result:
;   After HALT, memory at 0x0300 contains the sieve.
;   Primes ≤ 30: 2, 3, 5, 7, 11, 13, 17, 19, 23, 29
;   sieve[i] == 1 iff i is prime.
;
; Register map:
;   R0 = general scratch / loop variable
;   R1 = current prime p
;   R2 = multiple m (composite to mark)
;   R3 = scratch / address computation
;
; Cycle count: ~620 cycles
; ─────────────────────────────────────────────────────────────────────────────

; ── Step 1: Initialise sieve[0..30] = 1 ─────────────────────────────────────

        LOAD  R0, 0x0300   ; start address
        LOAD  R1, 1        ; value to write
        LOAD  R2, 31       ; count of elements

INIT_LOOP:
        LOAD  R3, 0
        CMP   R2, R3
        JZ    INIT_DONE
        STORE R1, R0       ; sieve[i] = 1
        ADDI  R0, 2        ; next address
        ADDI  R2, -1       ; count--
        JMP   INIT_LOOP

INIT_DONE:

; ── Step 2: Mark sieve[0] = 0 and sieve[1] = 0 ──────────────────────────────

        LOAD  R0, 0x0300
        LOAD  R1, 0
        STORE R1, R0       ; sieve[0] = 0

        LOAD  R0, 0x0302
        STORE R1, R0       ; sieve[1] = 0

; ── Step 3: Outer loop — p from 2 to 5 ──────────────────────────────────────
; We only need to sieve up to sqrt(30) ≈ 5
; R1 = p (current prime candidate)

        LOAD  R1, 2        ; p = 2

OUTER:
        LOAD  R3, 6        ; limit = sqrt(30) + 1 = 6
        CMP   R1, R3
        JZ    SIEVE_DONE   ; p >= 6 → done
        JN    CHECK_PRIME  ; p < 6 → continue
        JMP   SIEVE_DONE

CHECK_PRIME:
        ; addr of sieve[p] = 0x0300 + p*2
        MOV   R0, R1       ; R0 = p
        ADD   R0, R1       ; R0 = p*2
        LOAD  R3, 0x0300
        ADD   R0, R3       ; R0 = base + p*2
        LOADM R0, R0       ; R0 = sieve[p]

        LOAD  R3, 0
        CMP   R0, R3
        JZ    NEXT_P       ; sieve[p] == 0 → not prime, skip

        ; ── Inner loop: mark multiples of p starting at p*p ──────────────
        ; m = p * p
        MOV   R2, R1       ; R2 = p
        MUL   R2, R1       ; R2 = p*p  (m starts at p^2)

INNER:
        LOAD  R3, 31       ; upper limit
        CMP   R2, R3       ; if m > 30, done inner
        JN    MARK         ; m < 31 → mark
        JZ    NEXT_P       ; m == 31 → done inner
        JMP   NEXT_P       ; m > 31 → done inner

MARK:
        ; addr of sieve[m] = 0x0300 + m*2
        MOV   R3, R2       ; R3 = m
        ADD   R3, R2       ; R3 = m*2
        LOAD  R0, 0x0300
        ADD   R3, R0       ; R3 = base + m*2

        LOAD  R0, 0        ; value 0 (composite)
        STORE R0, R3       ; sieve[m] = 0

        ADD   R2, R1       ; m += p  (next multiple)
        JMP   INNER

NEXT_P:
        ADDI  R1, 1        ; p++
        JMP   OUTER

SIEVE_DONE:
        ; Memory 0x0300..0x033C now contains the sieve.
        ; Read with --debug and inspect memory region.
        ;
        ; Expected sieve[i] == 1 at i = 2,3,5,7,11,13,17,19,23,29
        ; All other indices should be 0.
        HALT

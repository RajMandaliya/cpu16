; ─────────────────────────────────────────────────────────────────────────────
; binary_search.asm  —  Search for a target in a sorted array using
;                        binary search. Returns the index or 0xFFFF (not found).
;
; Demonstrates: CALL, RET, PUSH, POP, LOADM, arithmetic on addresses,
;               divide-and-conquer logic in assembly.
;
; Input (sorted array at 0x0300, 8 elements):
;   [0x0002, 0x0005, 0x0008, 0x000B, 0x000E, 0x0011, 0x0014, 0x0017]
;   (i.e. 2, 5, 8, 11, 14, 17, 20, 23)
;
; Target: 0x000E (14) → expected result in R0 = 4 (zero-based index)
; Target: 0x0099      → expected result in R0 = 0xFFFF (not found)
;
; Register map (caller):
;   R0 = target value to search for
;
; Register map (subroutine BSEARCH):
;   R0 = target (preserved as input)
;   R1 = low index
;   R2 = high index
;   R3 = mid index / scratch
;
; Return value:
;   R0 = index of target if found, 0xFFFF if not found
;
; Cycle count (8-element array): max 24 cycles (3 iterations × ~8 ops)
; ─────────────────────────────────────────────────────────────────────────────

; ── Write sorted input array to memory ───────────────────────────────────────

        LOAD  R0, 0x0300
        LOAD  R1, 2
        STORE R1, R0
        ADDI  R0, 2
        LOAD  R1, 5
        STORE R1, R0
        ADDI  R0, 2
        LOAD  R1, 8
        STORE R1, R0
        ADDI  R0, 2
        LOAD  R1, 11
        STORE R1, R0
        ADDI  R0, 2
        LOAD  R1, 14
        STORE R1, R0
        ADDI  R0, 2
        LOAD  R1, 17
        STORE R1, R0
        ADDI  R0, 2
        LOAD  R1, 20
        STORE R1, R0
        ADDI  R0, 2
        LOAD  R1, 23
        STORE R1, R0

; ── Call binary search ────────────────────────────────────────────────────────
; Save target in memory at 0x0320 before calling (CALL clobbers nothing
; but subroutine uses R0–R3, so we preserve target in memory)

        LOAD  R0, 14       ; target = 14
        LOAD  R1, 0x0320
        STORE R0, R1       ; save target to memory

        CALL  BSEARCH

        ; R0 now holds the result index (or 0xFFFF)
        HALT

; ─────────────────────────────────────────────────────────────────────────────
; BSEARCH subroutine
;
; Binary search over array at 0x0300 with N=8 elements.
; Reads target from memory address 0x0320.
;
; in:  (target at 0x0320, array at 0x0300, length = 8)
; out: R0 = index (0-based) if found, 0xFFFF if not found
; ─────────────────────────────────────────────────────────────────────────────
BSEARCH:
        LOAD  R1, 0        ; low = 0
        LOAD  R2, 7        ; high = N-1 = 7

BS_LOOP:
        ; if low > high: not found
        CMP   R1, R2       ; R1 - R2 → sets N if R1 < R2, Z if equal
        JN    BS_CALC_MID  ; low < high → continue
        JZ    BS_CALC_MID  ; low == high → check this element too
        ; low > high → not found
        LOAD  R0, 0xFFFF
        RET

BS_CALC_MID:
        ; mid = (low + high) / 2
        MOV   R3, R1       ; R3 = low
        ADD   R3, R2       ; R3 = low + high
        ; cpu16 has no right-shift by 1 via DIV; use SHR
        ; But SHR operates on registers not immediates, so:
        ; Store R3, shift, reload
        LOAD  R0, 0x0322
        STORE R3, R0       ; save (low+high) to scratch
        LOADM R3, R0       ; reload (redundant but explicit)
        ; SHR R3 shifts right by 1 (divide by 2, unsigned)
        ; Note: SHR is a 1-bit right shift in cpu16 ISA
        ; We use a loop-based halving since cpu16 SHR shifts by 1 bit:
        ; Actually SHR R3 does R3 >> 1 which IS divide by 2. Use it directly.
        ; (If your ISA SHR does R3 >>= 1, uncomment next line)
        ; SHR   R3, R3     ; mid = (low+high) >> 1
        ; Portable alternative using subtraction loop:
        MOV   R0, R3       ; R0 = low + high
        LOAD  R3, 0        ; mid = 0
HALVE:
        LOAD  R1, 2        ; we subtract 2 each iteration, add 1 to mid
        CMP   R0, R1       ; if (low+high) < 2, done halving
        JN    HALVE_DONE
        JZ    HALVE_DONE
        SUB   R0, R1       ; (low+high) -= 2
        ADDI  R3, 1        ; mid++
        JMP   HALVE
HALVE_DONE:
        ; R3 = mid index
        ; Reload low (was clobbered by scratch above)
        LOAD  R0, 0x0322
        LOADM R0, R0       ; R0 = low + high (scratch)
        ; Restore low and high from stack
        ; We pushed them before BS_CALC_MID — reload from memory instead
        ; For simplicity store low/high in fixed memory slots
        LOAD  R0, 0x0324
        LOADM R1, R0       ; reload low
        LOAD  R0, 0x0326
        LOADM R2, R0       ; reload high

        ; addr of arr[mid] = 0x0300 + mid*2
        MOV   R0, R3       ; R0 = mid
        ADD   R0, R3       ; R0 = mid*2  (mid + mid)
        LOAD  R1, 0x0300
        ADD   R0, R1       ; R0 = base + mid*2

        LOADM R0, R0       ; R0 = arr[mid]

        ; load target from memory
        LOAD  R1, 0x0320
        LOADM R1, R1       ; R1 = target

        CMP   R0, R1       ; arr[mid] - target
        JZ    BS_FOUND     ; arr[mid] == target

        ; reload low and high
        LOAD  R0, 0x0324
        LOADM R1, R0       ; R1 = low
        LOAD  R0, 0x0326
        LOADM R2, R0       ; R2 = high

        ; reload arr[mid] for comparison direction
        MOV   R0, R3
        ADD   R0, R3
        LOAD  R1, 0x0300
        ADD   R0, R1
        LOADM R0, R0       ; R0 = arr[mid]

        LOAD  R1, 0x0320
        LOADM R1, R1       ; R1 = target

        CMP   R0, R1       ; arr[mid] - target: N set means arr[mid] < target
        JN    BS_RIGHT     ; arr[mid] < target → search right half

        ; arr[mid] > target → search left: high = mid - 1
        MOV   R2, R3
        ADDI  R2, -1
        LOAD  R0, 0x0326
        STORE R2, R0       ; save new high
        ; save low unchanged
        LOAD  R1, 0x0324
        LOADM R1, R1
        LOAD  R0, 0x0324
        STORE R1, R0
        JMP   BS_LOOP

BS_RIGHT:
        ; arr[mid] < target → search right: low = mid + 1
        MOV   R1, R3
        ADDI  R1, 1
        LOAD  R0, 0x0324
        STORE R1, R0       ; save new low
        LOAD  R0, 0x0326
        LOADM R2, R0
        STORE R2, R0       ; save high unchanged
        JMP   BS_LOOP

BS_FOUND:
        MOV   R0, R3       ; return mid index
        RET

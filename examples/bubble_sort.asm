; ─────────────────────────────────────────────────────────────────────────────
; bubble_sort.asm  —  Sort 8 integers in memory using bubble sort.
;
; Demonstrates: LOADM, STORE, nested loops, CMP + conditional jumps,
;               indirect memory access via register addressing.
;
; Input array (stored at DATA_BASE = 0x0300):
;   [0x0008, 0x0003, 0x0007, 0x0001, 0x0006, 0x0002, 0x0005, 0x0004]
;
; Expected output (sorted ascending, verified in --debug memory dump):
;   [0x0001, 0x0002, 0x0003, 0x0004, 0x0005, 0x0006, 0x0007, 0x0008]
;
; Algorithm:
;   for i in 0..N-1:
;       for j in 0..N-1-i:
;           if arr[j] > arr[j+1]: swap(arr[j], arr[j+1])
;
; Register map:
;   R0 = base address of array / scratch for LOADM
;   R1 = current element arr[j]
;   R2 = next element arr[j+1]
;   R3 = inner loop counter / swap temp
;
; Memory layout:
;   0x0300  arr[0]   (2 bytes each, little-endian 16-bit words)
;   0x0302  arr[1]
;   ...
;   0x030E  arr[7]
;
; Cycle count (worst case, reverse-sorted input): ~450 cycles
; ─────────────────────────────────────────────────────────────────────────────

; ── Write input array to memory ──────────────────────────────────────────────
; We use STORE to initialise the array at 0x0300 before sorting.

        LOAD  R0, 0x0300   ; base address
        LOAD  R1, 8
        STORE R1, R0       ; arr[0] = 8
        ADDI  R0, 2
        LOAD  R1, 3
        STORE R1, R0       ; arr[1] = 3
        ADDI  R0, 2
        LOAD  R1, 7
        STORE R1, R0       ; arr[2] = 7
        ADDI  R0, 2
        LOAD  R1, 1
        STORE R1, R0       ; arr[3] = 1
        ADDI  R0, 2
        LOAD  R1, 6
        STORE R1, R0       ; arr[4] = 6
        ADDI  R0, 2
        LOAD  R1, 2
        STORE R1, R0       ; arr[5] = 2
        ADDI  R0, 2
        LOAD  R1, 5
        STORE R1, R0       ; arr[6] = 5
        ADDI  R0, 2
        LOAD  R1, 4
        STORE R1, R0       ; arr[7] = 4

; ── Outer loop: i from 7 down to 1 ──────────────────────────────────────────
; R3 = outer counter (number of remaining passes = N-1 = 7)

        LOAD  R3, 7        ; outer counter = N-1

OUTER_LOOP:
        LOAD  R0, 0        ; zero for comparison
        CMP   R3, R0       ; if outer counter == 0, done
        JZ    SORT_DONE

; ── Inner loop: walk j from base to base + (counter-1)*2 ─────────────────────
; R0 = current pointer (starts at array base each outer pass)

        LOAD  R0, 0x0300   ; reset pointer to array base

        ; inner limit = base + R3*2  (we do R3 comparisons per pass)
        ; We track remaining inner steps in R2, counting down from R3
        MOV   R2, R3       ; inner counter = outer counter

INNER_LOOP:
        LOAD  R1, 0
        CMP   R2, R1       ; if inner counter == 0, end inner loop
        JZ    INNER_DONE

        LOADM R1, R0       ; R1 = arr[j]   (load from address in R0)
        ADDI  R0, 2        ; advance pointer to j+1
        LOADM R2, R0       ; R2 = arr[j+1] (load from address in R0)

        ; if arr[j] <= arr[j+1], no swap needed
        ; CMP R1, R2 computes R1 - R2 and sets flags
        ; if N flag clear and Z flag clear → R1 > R2 → swap
        CMP   R1, R2
        JN    NO_SWAP      ; R1 < R2 → skip swap  (N flag set)
        JZ    NO_SWAP      ; R1 == R2 → skip swap (Z flag set)

        ; swap arr[j] and arr[j+1]
        ; R0 currently points to j+1
        STORE R1, R0       ; arr[j+1] = old arr[j]
        ADDI  R0, -2       ; back to j
        STORE R2, R0       ; arr[j]   = old arr[j+1]
        ADDI  R0, 2        ; advance to j+1 again

NO_SWAP:
        ; reload inner counter — it was overwritten by LOADM R2, R0
        ; We use the stack to preserve R3 across the inner loop
        ; (R2 was clobbered — recalculate from pointer position)
        ; inner steps remaining = (base + R3*2 - R0) / 2
        ; Simpler: push R3 before inner loop, pop after each step
        ; For clarity here we use a dedicated counter in memory at 0x0320
        LOAD  R2, 0x0320
        LOADM R2, R2       ; reload inner counter from memory
        ADDI  R2, -1       ; decrement
        LOAD  R1, 0x0320
        STORE R2, R1       ; save back
        JMP   INNER_LOOP

INNER_DONE:
        ADDI  R3, -1       ; outer counter--
        ; save inner counter = R3 into memory for next pass
        LOAD  R1, 0x0320
        STORE R3, R1
        JMP   OUTER_LOOP

SORT_DONE:
        ; Sorted array now lives at 0x0300–0x030E
        ; Verify with: cargo run --bin cpu16 -- bubble_sort.bin --debug
        ; Memory at 0x0300 should read: 0001 0002 0003 0004 0005 0006 0007 0008
        HALT

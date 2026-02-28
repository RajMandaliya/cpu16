; ─────────────────────────────────────────────────────────────────────────────
; interrupt_demo.asm  —  Software interrupt demonstration.
;
; Sets up a vector for INT 0, then triggers it.
; The handler increments a counter in R3 and returns.
;
; Memory layout:
;   0x0000 = IVT slot 0  (2 bytes) → points to HANDLER
;   0x0200 = program start
; ─────────────────────────────────────────────────────────────────────────────

; ── Write IVT entry for interrupt 0 ──────────────────────────────────────────
; We can't use STORE with an immediate address directly, so load the
; handler address and the IVT address into registers first.

        LOAD  R0, 0x0000   ; IVT slot 0 address
        LOAD  R1, HANDLER  ; address of our handler

        STORE R0, R1       ; mem[0x0000] = HANDLER address

; ── Main program ──────────────────────────────────────────────────────────────
        LOAD  R3, 0        ; interrupt counter = 0
        EI                 ; enable interrupts

        INT   0            ; trigger software interrupt 0
        INT   0            ; trigger again

        ; R3 should be 2 after two interrupts
        HALT

; ── Interrupt handler ─────────────────────────────────────────────────────────
HANDLER:
        ADDI  R3, 1        ; increment counter
        IRET               ; return from interrupt
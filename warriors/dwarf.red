;redcode-94
;name Dwarf
;author A.K. Dewdney
;strategy Bombs the core with DAT instructions at intervals of 4.

        ADD.AB #4, $3
        MOV.I  $2, @2
        JMP.B  $-2, $0
        DAT.F  #0, #0

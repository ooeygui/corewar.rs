;redcode-94
;name Mice
;author Chip Wendell
;strategy Replicator - copies itself to a new location and jumps there.

        MOV.AB #12, $-1
gate    MOV.I  @-2, <5
        DJN.F  $-1, $-3
        SPL.B  @3, $0
        ADD.AB #653, $2
        JMZ.F  $-5, $-6
ptr     DAT.F  #0, #833

;redcode-94
;name Test Warrior
;author Developer
;strategy Simple bomber

step    EQU 4
CORESIZE EQU 8000

loop    ADD.AB  #step, target
        MOV.I   bomb, @target
        JMP.B   loop
target  DAT.F   #0, #0
bomb    DAT.F   #0, #0

        ORG loop

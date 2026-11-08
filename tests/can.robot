*** Settings ***
Resource            apus.resource

*** Variables ***
${TRANSFERS}        0.01
# RX1 (FMPIE1, bit 4) left out: in Renode 1.17.0 the CPU stalls on entering the handler for it
&{SOURCES}          TX=0    RX0=1    SCE=17

*** Test Cases ***
Each Can Interrupt Should Reach The Cpu
    FOR    ${source}    ${ier_bit}    IN    &{SOURCES}
        Create Machine          can_irq
        Write Symbol            SOURCE    ${ier_bit}
        Execute Command         emulation RunFor "${TRANSFERS}"
        ${entries}=             Read Symbol    ENTRIES
        Should Be True          ${entries} > 0    no CEC_CAN interrupt from ${source}
        Reset Emulation
    END

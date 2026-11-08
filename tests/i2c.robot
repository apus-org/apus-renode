*** Settings ***
Resource            i2c.resource

*** Test Cases ***
I2c1 Interrupt Should Reach The Cpu
    Create Machine              i2c_irq
    ${entries}=                 Read Symbol    ENTRIES
    Should Be Equal As Integers    ${entries}    1

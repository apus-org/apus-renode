*** Settings ***
Resource            i2c.resource

*** Test Cases ***
I2c1 Interrupt Should Reach The Cpu
    Create Machine              i2c_irq
    ${entries}=                 Read Symbol    ENTRIES
    Should Be Equal As Integers    ${entries}    1

Board Reset Should Keep Device State
    Create Machine              i2c_bridge
    Write Bytes                 ${RTC}    0x20    0x5A
    Write Bytes                 ${RTC}    0x07    0x00
    Write Bytes                 ${RTC}    0x00    0x80
    Write Bytes                 ${EEPROM}    0x10    0xA5
    Run                         2.5
    Execute Command             machine Reset
    Run                         ${STARTUP}
    Bytes Should Be             ${RTC}    0x20    0x5A
    Bytes Should Be             ${RTC}    0x00    0x82
    Bytes Should Be             ${EEPROM}    0x10    0xA5
    Line Should Be              mfp    0
    Line Should Be              alert    1
    Write Bytes                 ${RTC}    0x07    0x80
    Execute Command             machine Reset
    Run                         ${STARTUP}
    Line Should Be              mfp    1

*** Settings ***
Resource            i2c.resource
Test Setup          Create Machine    i2c_bridge

*** Keywords ***
Unlock
    Write Bytes                 ${RTC}    0x09    0x55
    Write Bytes                 ${RTC}    0x09    0xAA

*** Test Cases ***
Memory Should Start Erased
    Bytes Should Be             ${EEPROM}    0x00    0xFF    0xFF    0xFF    0xFF    0xFF    0xFF    0xFF    0xFF
    Bytes Should Be             ${EEPROM}    0xF0    0xFF
    Bytes Should Be             ${EEPROM}    0xFF    0x00

Page Write Should Wrap Within The Page
    Write Bytes                 ${EEPROM}    0x06    0x01    0x02    0x03    0x04
    Bytes Should Be             ${EEPROM}    0x00    0x03    0x04    0xFF    0xFF    0xFF    0xFF    0x01    0x02
    Write Bytes                 ${EEPROM}    0x08    0x01    0x02    0x03    0x04    0x05    0x06    0x07    0x08    0x09    0x0A
    Bytes Should Be             ${EEPROM}    0x08    0x09    0x0A    0x03    0x04    0x05    0x06    0x07    0x08

Sequential Read Should Wrap At The End
    Write Bytes                 ${EEPROM}    0x7F    0x77
    Write Bytes                 ${EEPROM}    0x00    0x11
    Bytes Should Be             ${EEPROM}    0x7F    0x77    0x11

Pointer Should Be Shared With The Rtc
    Write Bytes                 ${EEPROM}    0x30    0x42
    Write Bytes                 ${RTC}    0x30
    ${bytes}=                   Transfer    ${EEPROM}    ${{[]}}    1
    Should Be Equal             ${bytes}    ${{[0x42]}}

Block Protection Should Drop Writes
    FOR    ${protect}    ${first}    IN    0x04    0x60    0x08    0x40
        Write Bytes             ${EEPROM}    0xFF    ${protect}
        Bytes Should Be         ${EEPROM}    0xFF    ${protect}
        Write Bytes             ${EEPROM}    ${{int($first, 0) - 1}}    0x01
        Write Bytes             ${EEPROM}    ${first}    0x01
        Bytes Should Be         ${EEPROM}    ${{int($first, 0) - 1}}    0x01
        Bytes Should Be         ${EEPROM}    ${first}    0xFF
    END
    Write Bytes                 ${EEPROM}    0xFF    0x0C
    Write Bytes                 ${EEPROM}    0x00    0x01
    Bytes Should Be             ${EEPROM}    0x00    0xFF

Protected Block Should Need The Unlock Sequence
    Write Bytes                 ${EEPROM}    0xF0    0x12
    Bytes Should Be             ${EEPROM}    0xF0    0xFF
    Write Bytes                 ${RTC}    0x09    0x55
    Bytes Should Be             ${RTC}    0x09    0x00
    Write Bytes                 ${RTC}    0x09    0xAA
    Write Bytes                 ${EEPROM}    0xF0    0x12
    Bytes Should Be             ${EEPROM}    0xF0    0xFF
    Unlock
    Write Bytes                 ${EEPROM}    0xF0    0x12
    Bytes Should Be             ${EEPROM}    0xF0    0x12
    Write Bytes                 ${EEPROM}    0xF1    0x34
    Bytes Should Be             ${EEPROM}    0xF1    0xFF
    Unlock
    Execute Command             machine Reset
    Run                         ${STARTUP}
    Write Bytes                 ${EEPROM}    0xF1    0x34
    Bytes Should Be             ${EEPROM}    0xF1    0xFF

*** Settings ***
Resource            i2c.resource
Test Setup          Create Machine    i2c_bridge

*** Keywords ***
Set Time And Run
    [Arguments]                 @{time}
    Write Bytes                 ${RTC}    0x00    @{time}
    Run                         1.5

*** Test Cases ***
Registers Should Start At Power-On Values
    Bytes Should Be             ${RTC}    0x00    0x00    0x00    0x00    0x01    0x01    0x01    0x01    0x80
    Run                         1.5
    Bytes Should Be             ${RTC}    0x00    0x00
    Line Should Be              mfp    1

Oscillator Should Count Seconds
    Write Bytes                 ${RTC}    0x00    0x80
    Bytes Should Be             ${RTC}    0x03    0x21
    Run                         2.5
    Bytes Should Be             ${RTC}    0x00    0x82

Time Should Carry Into Date And Year
    Set Time And Run            0xD9    0x59    0x23    0x07    0x31    0x12    0x99
    Bytes Should Be             ${RTC}    0x00    0x80    0x00    0x00    0x21    0x01    0x21    0x00

February Should Follow Leap Years
    Set Time And Run            0xD9    0x59    0x23    0x01    0x28    0x02    0x23
    Bytes Should Be             ${RTC}    0x04    0x01    0x03
    Set Time And Run            0xD9    0x59    0x23    0x01    0x28    0x02    0x24
    Bytes Should Be             ${RTC}    0x04    0x29    0x22

Address Pointer Should Wrap Per Block
    Write Bytes                 ${RTC}    0x5F    0x11    0x22
    Bytes Should Be             ${RTC}    0x5F    0x11    0x22
    Write Bytes                 ${RTC}    0x1E    0x12    0x07    0x33
    Bytes Should Be             ${RTC}    0x1F    0x07    0x33

General Purpose Output Should Drive Mfp
    Write Bytes                 ${RTC}    0x07    0x00
    Line Should Be              mfp    0
    Write Bytes                 ${RTC}    0x07    0x80
    Line Should Be              mfp    1

Enabled Alarm Should Latch Its Flag And Drive Mfp
    Write Bytes                 ${RTC}    0x0A    0x02
    Write Bytes                 ${RTC}    0x11    0x02
    Write Bytes                 ${RTC}    0x07    0x90
    Write Bytes                 ${RTC}    0x00    0x80
    Run                         1.5
    Bytes Should Be             ${RTC}    0x0D    0x01
    Line Should Be              mfp    1
    Run                         1.0
    Bytes Should Be             ${RTC}    0x0D    0x09
    Bytes Should Be             ${RTC}    0x14    0x01
    Line Should Be              mfp    0
    Write Bytes                 ${RTC}    0x0D    0x01
    Bytes Should Be             ${RTC}    0x0D    0x09
    Run                         1.0
    Bytes Should Be             ${RTC}    0x0D    0x09
    Line Should Be              mfp    0
    Write Bytes                 ${RTC}    0x0D    0x09
    Bytes Should Be             ${RTC}    0x0D    0x01
    Line Should Be              mfp    1

Active-High Alarm Should Raise Mfp
    Write Bytes                 ${RTC}    0x0A    0x01    0x00    0x00    0x81
    Write Bytes                 ${RTC}    0x07    0x10
    Line Should Be              mfp    0
    Write Bytes                 ${RTC}    0x00    0x80
    Run                         1.5
    Line Should Be              mfp    1

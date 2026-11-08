*** Settings ***
Resource            i2c.resource
Test Setup          Create Machine    i2c_bridge

*** Variables ***
${CONVERSION}       0.035
${SLOWEST}          0.25

*** Keywords ***
Set Limits
    Write Bytes                 ${TEMPERATURE}    0x03    0x1E    0x00
    Write Bytes                 ${TEMPERATURE}    0x02    0x19    0x00

*** Test Cases ***
Registers Should Start At Power-On Values
    Bytes Should Be             ${TEMPERATURE}    0x01    0x00
    Bytes Should Be             ${TEMPERATURE}    0x02    0x4B    0x00
    Bytes Should Be             ${TEMPERATURE}    0x03    0x50    0x00
    Line Should Be              alert    1

Temperature Should Be Latched At Each Conversion
    Set Temperature             21.5
    Bytes Should Be             ${TEMPERATURE}    0x00    0x00    0x00
    Run                         ${CONVERSION}
    Bytes Should Be             ${TEMPERATURE}    0x00    0x15    0x80

Resolution Should Set The Fraction Bits
    Set Temperature             21.9375
    FOR    ${resolution}    ${fraction}    IN    0    0x80    1    0xC0    2    0xE0    3    0xF0
        Write Bytes             ${TEMPERATURE}    0x01    ${{int($resolution) << 5}}
        Run                     ${SLOWEST}
        Bytes Should Be         ${TEMPERATURE}    0x00    0x15    ${fraction}
    END

Negative Temperature Should Be Twos Complement
    Set Temperature             -0.5
    Run                         ${CONVERSION}
    Bytes Should Be             ${TEMPERATURE}    0x00    0xFF    0x80

Limits Should Keep Nine Bits
    Write Bytes                 ${TEMPERATURE}    0x02    0x12    0xFF
    Bytes Should Be             ${TEMPERATURE}    0x02    0x12    0x80

Comparator Alert Should Follow Limit And Hysteresis
    Set Limits
    Set Temperature             31
    Run                         ${CONVERSION}
    Line Should Be              alert    0
    Set Temperature             27
    Run                         ${CONVERSION}
    Line Should Be              alert    0
    Set Temperature             24
    Run                         ${CONVERSION}
    Line Should Be              alert    1

Active-High Alert Should Invert The Line
    Write Bytes                 ${TEMPERATURE}    0x01    0x04
    Line Should Be              alert    0
    Set Limits
    Set Temperature             31
    Run                         ${CONVERSION}
    Line Should Be              alert    1

Interrupt Alert Should Clear On Read
    Write Bytes                 ${TEMPERATURE}    0x01    0x02
    Set Limits
    Set Temperature             31
    Run                         ${CONVERSION}
    Line Should Be              alert    0
    Run                         ${CONVERSION}
    Line Should Be              alert    0
    Read Bytes                  ${TEMPERATURE}    0x00    2
    Line Should Be              alert    1
    Run                         ${CONVERSION}
    Line Should Be              alert    1
    Set Temperature             24
    Run                         ${CONVERSION}
    Line Should Be              alert    0
    Read Bytes                  ${TEMPERATURE}    0x00    2
    Line Should Be              alert    1

Fault Queue Should Delay The Alert
    Write Bytes                 ${TEMPERATURE}    0x01    0x10
    Set Limits
    Set Temperature             31
    Run                         0.075
    Line Should Be              alert    1
    Run                         0.06
    Line Should Be              alert    0

Shutdown Should Freeze The Temperature Until One-Shot
    Set Temperature             21.5
    Run                         ${CONVERSION}
    Write Bytes                 ${TEMPERATURE}    0x01    0x01
    Set Temperature             30
    Run                         ${CONVERSION}
    Bytes Should Be             ${TEMPERATURE}    0x00    0x15    0x80
    Write Bytes                 ${TEMPERATURE}    0x01    0x81
    Bytes Should Be             ${TEMPERATURE}    0x01    0x81
    Run                         ${CONVERSION}
    Bytes Should Be             ${TEMPERATURE}    0x01    0x01
    Bytes Should Be             ${TEMPERATURE}    0x00    0x1E    0x00

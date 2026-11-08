*** Settings ***
Resource            apus.resource
Test Setup          Create Machine    touch

*** Variables ***
${TOUCH}            sysbus.spi1.touch
${ATTEMPTS}         100

*** Keywords ***
Frame
    [Arguments]                 @{bytes}    ${hal}=1    ${deselected}=0    ${hold}=0
    ${base}=                    Symbol Address    TX
    FOR    ${index}    ${byte}    IN ENUMERATE    @{bytes}
        Execute Command         sysbus WriteByte ${{int($base, 0) + $index}} ${byte}
    END
    ${len}=                     Get Length    ${bytes}
    Store Symbol                LEN    ${len}
    Store Symbol                HAL    ${hal}
    Store Symbol                DESELECTED    ${deselected}
    Store Symbol                HOLD    ${hold}
    ${request}=                 Read Symbol    REQUEST
    ${request}=                 Evaluate    $request + 1
    Store Symbol                REQUEST    ${request}
    FOR    ${attempt}    IN RANGE    ${ATTEMPTS}
        Execute Command         emulation RunFor "${STEP}"
        ${done}=                Read Symbol    DONE
        IF    ${done} == ${request}    BREAK
    END
    Should Be Equal As Integers    ${done}    ${request}    frame not sent
    ${failed}=                  Read Symbol    FAILED
    Should Be Equal As Integers    ${failed}    0    frame timed out
    ${base}=                    Symbol Address    RX
    ${received}=                Evaluate    []
    FOR    ${index}    IN RANGE    ${len}
        ${byte}=                Execute Command    sysbus ReadByte ${{int($base, 0) + $index}}
        Evaluate                $received.append(int($byte, 0))
    END
    RETURN                      ${received}

Convert
    [Arguments]                 ${command}
    ${received}=                Frame    ${command}    0x00    0x00
    RETURN                      ${{($received[1] << 8 | $received[2]) >> 3 & 0xFFF}}

Conversion Should Be
    [Arguments]                 ${command}    ${expected}
    ${value}=                   Convert    ${command}
    Should Be Equal As Integers    ${value}    ${expected}    conversion ${command}

Touch
    [Arguments]                 ${x}    ${y}
    Execute Command             ${TOUCH} MoveTo ${x} ${y}
    Execute Command             ${TOUCH} Press

Release Touch
    Execute Command             ${TOUCH} Release

Pen Should Be
    [Arguments]                 ${level}
    Execute Command             emulation RunFor "${STEP}"
    ${pins}=                    Read Symbol    PINS
    Should Be Equal As Integers    ${pins}    ${level}    PENIRQ

*** Test Cases ***
Corners Should Read The Measured Edges
    Touch                       0    0
    Conversion Should Be        0xD0    202
    Conversion Should Be        0x90    3837
    Touch                       239    319
    Conversion Should Be        0xD0    3820
    Conversion Should Be        0x90    264

Middle Should Map Linearly
    Touch                       120    160
    Conversion Should Be        0xD0    2019
    Conversion Should Be        0x90    2045

Pen Interrupt Should Follow The Touch
    Pen Should Be               1
    Touch                       10    10
    Pen Should Be               0
    Release Touch
    Pen Should Be               1

Deselected Controller Should Ignore The Bus
    Touch                       0    0
    ${received}=                Frame    0xD0    0x00    0x00    hal=0    deselected=1
    Should Be Equal             ${received}    ${{[0, 0, 0]}}
    Conversion Should Be        0xD0    202

Pressure Should Tell Touch From None
    Conversion Should Be        0xD0    0
    Conversion Should Be        0xB0    0
    Conversion Should Be        0xC0    4095
    Touch                       10    10
    Conversion Should Be        0xB0    2048
    Conversion Should Be        0xC0    2560

Power-Down Bit Should Switch The Pen Interrupt
    Touch                       10    10
    Convert                     0xD1
    Pen Should Be               1
    Convert                     0xD0
    Pen Should Be               0

Pen Interrupt Should Show The Conversion
    Frame                       0xD0    hold=1
    Pen Should Be               0
    Frame                       deselected=1
    Pen Should Be               1
    Frame                       0xD0    0x00    0x00    hold=1
    Pen Should Be               1
    Frame                       deselected=1
    Touch                       10    10
    Frame                       0x84    hold=1
    Pen Should Be               1
    Frame                       deselected=1
    Pen Should Be               0

Overlapped Commands Should Return Both Results
    Touch                       0    0
    ${received}=                Frame    0xD0    0x00    0x90    0x00    0x00    hal=0
    Should Be Equal As Integers    ${{($received[1] << 8 | $received[2]) >> 3 & 0xFFF}}    202
    Should Be Equal As Integers    ${{($received[3] << 8 | $received[4]) >> 3 & 0xFFF}}    3837

Command In The First Result Byte Should Be Ignored
    Touch                       0    0
    ${received}=                Frame    0xD0    0x90    0x00    0x00    hal=0
    Should Be Equal As Integers    ${{($received[1] << 8 | $received[2]) >> 3 & 0xFFF}}    202
    Should Be Equal As Integers    ${received}[3]    0

Aborted Conversion Should Not Leak Into The Next Frame
    Touch                       0    0
    Frame                       0xD0    0x00
    Conversion Should Be        0x90    3837

Unmodelled Conversions Should Read Zero
    Touch                       0    0
    Conversion Should Be        0xD8    0
    Conversion Should Be        0x84    0

Pen Interrupt Should Survive A Board Reset
    Touch                       10    10
    Pen Should Be               0
    Execute Command             machine Reset
    Execute Command             emulation RunFor "${STARTUP}"
    Pen Should Be               0
    Release Touch
    Execute Command             machine Reset
    Execute Command             emulation RunFor "${STARTUP}"
    Pen Should Be               1

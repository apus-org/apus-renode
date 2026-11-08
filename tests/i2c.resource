*** Settings ***
Resource            apus.resource
Library             Collections

*** Variables ***
${TEMPERATURE}      0x48
${RTC}              0x6F
${EEPROM}           0x57
${ATTEMPTS}         100

*** Keywords ***
Transfer
    [Arguments]                 ${address}    ${write}    ${read_len}=0    ${separate}=0
    Store Symbol                ADDRESS    ${address}
    ${base}=                    Symbol Address    WRITE
    FOR    ${index}    ${byte}    IN ENUMERATE    @{write}
        Execute Command         sysbus WriteByte ${{int($base, 0) + $index}} ${byte}
    END
    ${write_len}=               Get Length    ${write}
    Should Be True              ${write_len} <= 32 and ${read_len} <= 32    the bridge moves at most 32 bytes each way
    Store Symbol                WRITE_LEN    ${write_len}
    Store Symbol                READ_LEN    ${read_len}
    Store Symbol                SEPARATE    ${separate}
    ${request}=                 Read Symbol    REQUEST
    ${request}=                 Evaluate    $request + 1
    Store Symbol                REQUEST    ${request}
    FOR    ${attempt}    IN RANGE    ${ATTEMPTS}
        Execute Command         emulation RunFor "${STEP}"
        ${done}=                Read Symbol    DONE
        IF    ${done} == ${request}    BREAK
    END
    Should Be Equal As Integers    ${done}    ${request}    transfer to ${address} did not end
    ${failed}=                  Read Symbol    FAILED
    Should Be Equal As Integers    ${failed}    0    transfer to ${address} timed out
    ${base}=                    Symbol Address    READ
    ${bytes}=                   Create List
    FOR    ${index}    IN RANGE    ${read_len}
        ${byte}=                Execute Command    sysbus ReadByte ${{int($base, 0) + $index}}
        Append To List          ${bytes}    ${{int($byte, 0)}}
    END
    RETURN                      ${bytes}

Write Bytes
    [Arguments]                 ${address}    @{bytes}
    Transfer                    ${address}    ${bytes}

Read Bytes
    [Arguments]                 ${address}    ${register}    ${count}=1
    ${bytes}=                   Transfer    ${address}    ${{[$register]}}    ${count}
    RETURN                      ${bytes}

Bytes Should Be
    [Arguments]                 ${address}    ${register}    @{expected}
    ${count}=                   Get Length    ${expected}
    ${bytes}=                   Read Bytes    ${address}    ${register}    ${count}
    ${expected}=                Evaluate    [int(byte, 0) for byte in $expected]
    Should Be Equal             ${bytes}    ${expected}    register ${register} of ${address}

Line Should Be
    [Arguments]                 ${line}    ${level}
    ${bit}=                     Set Variable If    "${line}" == "mfp"    ${0}    ${1}
    ${pins}=                    Read Symbol    PINS
    Should Be Equal As Integers    ${{($pins >> $bit) & 1}}    ${level}    ${line}

Set Temperature
    [Arguments]                 ${celsius}
    Execute Command             sysbus.i2c1.temperature Temperature ${celsius}

Run
    [Arguments]                 ${seconds}
    Execute Command             emulation RunFor "${seconds}"

*** Settings ***
Resource            apus.resource

*** Variables ***
${WINDOW}           0.1
${TICKS_PER_SECOND}    1000
@{USARTS}           usart1    usart2    usart3    usart4
${BAUD_RATE}        115200
${BAUD_TOLERANCE}   0.01

*** Test Cases ***
SysTick Should Tick Once Per Millisecond
    Create Machine              clocks
    ${start}=                   Read Symbol    MILLIS
    Execute Command             emulation RunFor "${WINDOW}"
    ${end}=                     Read Symbol    MILLIS
    ${expected}=                Evaluate    round(${WINDOW} * ${TICKS_PER_SECOND})
    ${error}=                   Evaluate    abs(${end} - ${start} - ${expected})
    Should Be True              ${error} <= 1    ${end} - ${start} ticks, expected ${expected}

Usarts Should Run At The Configured Baud Rate
    Create Machine              clocks
    FOR    ${usart}    IN    @{USARTS}
        ${measured}=            Execute Command    sysbus.${usart} BaudRate
        ${error}=               Evaluate    abs(int($measured, 0) - ${BAUD_RATE}) / ${BAUD_RATE}
        Should Be True          ${error} < ${BAUD_TOLERANCE}    ${usart}: ${measured}
    END

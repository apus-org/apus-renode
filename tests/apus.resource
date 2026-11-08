*** Variables ***
${FIRMWARE}         ${CURDIR}/firmware/target/thumbv6m-none-eabi/release
${SCRIPT}           ${CURDIR}/../scripts/apus.resc
${STARTUP}          0.01
${STEP}             0.001

*** Keywords ***
Create Machine
    [Arguments]                 ${binary}
    Execute Command             $bin=@${FIRMWARE}/${binary}
    Execute Command             include @${SCRIPT}
    Execute Command             emulation RunFor "${STARTUP}"

Symbol Address
    [Arguments]                 ${symbol}
    ${address}=                 Execute Command    sysbus GetSymbolAddress "${symbol}"
    ${address}=                 Evaluate    $address.strip()
    RETURN                      ${address}

Read Symbol
    [Arguments]                 ${symbol}
    ${address}=                 Symbol Address    ${symbol}
    ${value}=                   Execute Command    sysbus ReadDoubleWord ${address}
    ${value}=                   Evaluate    int($value, 0)
    RETURN                      ${value}

Store Symbol
    [Arguments]                 ${symbol}    ${value}
    ${address}=                 Symbol Address    ${symbol}
    Execute Command             sysbus WriteDoubleWord ${address} ${value}

Write Symbol
    [Arguments]                 ${symbol}    ${value}
    Store Symbol                ${symbol}    ${value}
    Execute Command             emulation RunFor "${STEP}"

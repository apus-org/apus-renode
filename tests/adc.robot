*** Settings ***
Resource            apus.resource

*** Variables ***
${VDDA}             3.3
${FULL_SCALE}       4095
${POT_VOLTS}        1.65
${VREFINT_VOLTS}    1.23
${VREFINT_CAL}      0x1FFFF7BA
${COUNTS}           2

*** Keywords ***
Raw Value Should Match
    [Arguments]                 ${symbol}    ${volts}
    ${raw}=                     Read Symbol    ${symbol}
    ${expected}=                Evaluate    round(${volts} / ${VDDA} * ${FULL_SCALE})
    ${error}=                   Evaluate    abs(${raw} - ${expected})
    Should Be True              ${error} <= ${COUNTS}    ${symbol}: ${raw}, expected ${expected}

*** Test Cases ***
Potentiometer Should Reach Channel 14
    Create Machine              adc_raw
    Execute Command             sysbus.adc.potentiometer Volts ${POT_VOLTS}
    Execute Command             emulation RunFor "${STEP}"
    Raw Value Should Match      POT    ${POT_VOLTS}

Internal Reference Should Reach Channel 17
    Create Machine              adc_raw
    Raw Value Should Match      VREFINT    ${VREFINT_VOLTS}

Calibration Should Hold The Internal Reference At 3.3 V
    Create Machine              adc_raw
    ${calibration}=             Execute Command    sysbus ReadWord ${VREFINT_CAL}
    ${expected}=                Evaluate    round(${VREFINT_VOLTS} / ${VDDA} * ${FULL_SCALE})
    Should Be Equal As Integers    ${calibration}    ${expected}

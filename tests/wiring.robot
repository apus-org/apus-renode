*** Settings ***
Resource            apus.resource

*** Variables ***
@{OUTPUTS}          led3    led4    led5    rgbRed    rgbGreen    rgbBlue
@{BUTTONS}          gpioPortA.wakeUp    gpioPortD.user0    gpioPortD.user1
${NONE}             0xFFFFFFFF

*** Keywords ***
Only Output Should Be On
    [Arguments]                 ${selected}
    FOR    ${output}    IN    @{OUTPUTS}
        ${state}=               Execute Command    sysbus.gpioPortC.${output} State
        ${expected}=            Evaluate    "${output}" == "${selected}"
        Should Be Equal         ${state.strip()}    ${expected.__str__()}    ${output}
    END

*** Test Cases ***
Each Output Should Drive Only Its Led
    Create Machine              wiring
    Write Symbol                OUTPUT    ${NONE}
    Only Output Should Be On    ${NONE}
    FOR    ${index}    ${output}    IN ENUMERATE    @{OUTPUTS}
        Write Symbol            OUTPUT    ${index}
        Only Output Should Be On    ${output}
    END

Each Button Should Drive Only Its Input
    Create Machine              wiring
    ${released}=                Read Symbol    BUTTONS
    Should Be Equal As Integers    ${released}    0
    FOR    ${bit}    ${button}    IN ENUMERATE    @{BUTTONS}
        Execute Command         sysbus.${button} Press
        Execute Command         emulation RunFor "${STEP}"
        ${pressed}=             Read Symbol    BUTTONS
        Should Be Equal As Integers    ${pressed}    ${1 << ${bit}}    ${button}
        Execute Command         sysbus.${button} Release
        Execute Command         emulation RunFor "${STEP}"
        ${released}=            Read Symbol    BUTTONS
        Should Be Equal As Integers    ${released}    0    ${button}
    END

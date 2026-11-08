*** Settings ***
Resource            apus.resource

*** Variables ***
@{OUTPUTS}          led3    led4    led5
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

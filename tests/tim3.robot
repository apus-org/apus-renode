*** Settings ***
Resource            apus.resource

*** Variables ***
@{CHANNEL_PINS}     rgbRed    rgbGreen    rgbBlue    buzzer
${WINDOW}           0.01
${PWM_FREQUENCY}    1000
${EDGE_TOLERANCE}   2

*** Keywords ***
Count Edges On
    [Arguments]                 ${pin}
    Execute Command             python "edges['${pin}'] = 0; self.Machine['sysbus.gpioPortC.${pin}'].StateChanged += lambda led, state: edges.__setitem__('${pin}', edges['${pin}'] + 1)"

Edges On
    [Arguments]                 ${pin}
    ${edges}=                   Execute Command    python "print(edges['${pin}'])"
    ${edges}=                   Evaluate    int($edges)
    RETURN                      ${edges}

*** Test Cases ***
Each Channel Should Reach Only Its Pin
    Create Machine              tim3_routes
    Execute Command             python "edges = {}"
    FOR    ${pin}    IN    @{CHANNEL_PINS}
        Count Edges On          ${pin}
    END
    ${expected}=                Evaluate    round(2 * ${PWM_FREQUENCY} * ${WINDOW})
    FOR    ${channel}    ${target}    IN ENUMERATE    @{CHANNEL_PINS}
        Write Symbol            CHANNEL    ${channel}
        Execute Command         python "for pin in list(edges): edges[pin] = 0"
        Execute Command         emulation RunFor "${WINDOW}"
        FOR    ${pin}    IN    @{CHANNEL_PINS}
            ${edges}=           Edges On    ${pin}
            IF    "${pin}" == "${target}"
                ${error}=       Evaluate    abs(${edges} - ${expected})
                Should Be True  ${error} <= ${EDGE_TOLERANCE}    CH${channel + 1} on ${pin}: ${edges} edges, expected ${expected}
            ELSE
                Should Be Equal As Integers    ${edges}    0    CH${channel + 1} leaked onto ${pin}
            END
        END
    END

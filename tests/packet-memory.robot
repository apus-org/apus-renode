*** Settings ***
Resource            apus.resource

*** Variables ***
${PACKET_MEMORY}    0x40006000

*** Test Cases ***
Packet Memory Should Keep What Is Written
    Create Machine              idle
    Execute Command             sysbus WriteDoubleWord ${PACKET_MEMORY} 0x12345678
    Execute Command             sysbus WriteWord ${{${PACKET_MEMORY} + 0x3FE}} 0xA55A
    ${first}=                   Execute Command    sysbus ReadDoubleWord ${PACKET_MEMORY}
    ${last}=                    Execute Command    sysbus ReadWord ${{${PACKET_MEMORY} + 0x3FE}}
    Should Be Equal As Integers    ${{int($first.strip(), 0)}}    0x12345678
    Should Be Equal As Integers    ${{int($last.strip(), 0)}}    0xA55A

*** Settings ***
Resource            apus.resource
Test Setup          Create Spi Machine    idle

*** Variables ***
${SPI2}             0x40003800
${CR1}              0x00
${CR2}              0x04
${SR}               0x08
${DR}               0x0C
${CRCPR}            0x10
${I2SPR}            0x20
# MSTR, SPE, SSI, SSM
${MASTER}           0x0344
# BIDIMODE, BIDIOE
${ONE_LINE}         0xC000
# FRXTH, 8-bit frames
${EIGHT_BIT}        0x1700
# 8-bit frames, RXNE at 16 bits
${HALF_WORD_RXNE}   0x0700
# FRXTH, 16-bit frames
${SIXTEEN_BIT}      0x1F00
${ERRIE}            0x0020
${RXNEIE}           0x0040
${TXEIE}            0x0080
${RXDMAEN}          0x0001
${DMA}              0x40020000
# SPI2 receive on channel 4
${CCR4}             0x44
${CNDTR4}           0x48
${CPAR4}            0x4C
${CMAR4}            0x50
${BUFFER}           0x20003000

*** Keywords ***
Create Spi Machine
    [Arguments]                 ${binary}
    Execute Command             $bin=@${FIRMWARE}/${binary}
    Execute Command             include @${SCRIPT}
    Execute Command             include @${CURDIR}/peripherals/SpiRecorder.cs
    Execute Command             machine LoadPlatformDescriptionFromString "recorder: SPI.SpiRecorder @ spi2"
    Execute Command             emulation RunFor "${STARTUP}"

Write Register
    [Arguments]                 ${offset}    ${value}
    Execute Command             sysbus WriteDoubleWord ${{${SPI2} + ${offset}}} ${value}

Read Register
    [Arguments]                 ${offset}
    ${value}=                   Execute Command    sysbus ReadDoubleWord ${{${SPI2} + ${offset}}}
    RETURN                      ${{int($value.strip(), 0)}}

Register Should Be
    [Arguments]                 ${offset}    ${expected}
    ${value}=                   Read Register    ${offset}
    Should Be Equal As Integers    ${value}    ${expected}    register ${offset}

Start
    [Arguments]                 ${control2}=${EIGHT_BIT}    ${control1}=${MASTER}
    Write Register              ${CR2}    ${control2}
    Write Register              ${CR1}    ${control1}

Send Byte
    [Arguments]                 ${value}
    Execute Command             sysbus WriteByte ${{${SPI2} + ${DR}}} ${value}

Send Word
    [Arguments]                 ${value}
    Execute Command             sysbus WriteWord ${{${SPI2} + ${DR}}} ${value}

Byte Should Arrive
    [Arguments]                 ${expected}
    ${value}=                   Execute Command    sysbus ReadByte ${{${SPI2} + ${DR}}}
    Should Be Equal As Integers    ${{int($value.strip(), 0)}}    ${expected}    byte read

Word Should Arrive
    [Arguments]                 ${expected}
    ${value}=                   Execute Command    sysbus ReadWord ${{${SPI2} + ${DR}}}
    Should Be Equal As Integers    ${{int($value.strip(), 0)}}    ${expected}    half-word read

Status Should Be
    [Arguments]                 ${rxne}=0    ${txe}=1    ${ovr}=0    ${frlvl}=0    ${ftlvl}=0
    ${sr}=                      Read Register    ${SR}
    Should Be Equal As Integers    ${{$sr & 1}}    ${rxne}    RXNE
    Should Be Equal As Integers    ${{$sr >> 1 & 1}}    ${txe}    TXE
    Should Be Equal As Integers    ${{$sr >> 6 & 1}}    ${ovr}    OVR
    Should Be Equal As Integers    ${{$sr >> 9 & 3}}    ${frlvl}    FRLVL
    Should Be Equal As Integers    ${{$sr >> 11 & 3}}    ${ftlvl}    FTLVL

Sent Should Be
    [Arguments]                 ${expected}
    ${sent}=                    Execute Command    sysbus.spi2.recorder Received
    Should Be Equal             ${sent.strip()}    ${expected}

Irq Should Be
    [Arguments]                 ${level}
    ${set}=                     Execute Command    sysbus.spi2 IRQ IsSet
    Should Be Equal             ${set.strip()}    ${level}

Write Dma Register
    [Arguments]                 ${offset}    ${value}
    Execute Command             sysbus WriteDoubleWord ${{${DMA} + ${offset}}} ${value}

Dma Should Have Left
    [Arguments]                 ${expected}
    ${value}=                   Execute Command    sysbus ReadDoubleWord ${{${DMA} + ${CNDTR4}}}
    Should Be Equal As Integers    ${{int($value.strip(), 0)}}    ${expected}    DMA count

Buffer Should Hold
    [Arguments]                 @{expected}
    FOR    ${index}    ${byte}    IN ENUMERATE    @{expected}
        ${value}=               Execute Command    sysbus ReadByte ${{${BUFFER} + ${index}}}
        Should Be Equal As Integers    ${{int($value.strip(), 0)}}    ${byte}    buffer ${index}
    END

Word Symbol Should Be
    [Arguments]                 ${symbol}    ${index}    ${expected}
    ${address}=                 Symbol Address    ${symbol}
    ${value}=                   Execute Command    sysbus ReadDoubleWord ${{int($address, 0) + 4 * ${index}}}
    Should Be Equal As Integers    ${{int($value.strip(), 0)}}    ${expected}    ${symbol} ${index}

Byte Symbol Should Be
    [Arguments]                 ${symbol}    ${index}    ${expected}
    ${address}=                 Symbol Address    ${symbol}
    ${value}=                   Execute Command    sysbus ReadByte ${{int($address, 0) + ${index}}}
    Should Be Equal As Integers    ${{int($value.strip(), 0)}}    ${expected}    ${symbol} ${index}

*** Test Cases ***
Registers Should Start At Their Reset Values
    Register Should Be          ${CR1}    0x0000
    Register Should Be          ${CR2}    0x0700
    Register Should Be          ${SR}    0x0002
    Register Should Be          ${CRCPR}    0x0007
    Register Should Be          ${I2SPR}    0x0002

Reserved Data Sizes Should Read As Eight Bits
    Write Register              ${CR2}    0x0200
    Register Should Be          ${CR2}    0x0700

Byte Write Should Send One Frame
    Start
    Send Byte                   0x41
    Sent Should Be              41
    Status Should Be            rxne=1    frlvl=1
    Byte Should Arrive          0x42
    Status Should Be

Half-Word Write Should Send Two Frames Low Byte First
    Start
    Send Word                   0x2211
    Sent Should Be              11 22
    Status Should Be            rxne=1    frlvl=2
    Word Should Arrive          0x2312
    Status Should Be

Rxne Should Follow The Threshold
    Start                       ${HALF_WORD_RXNE}
    Send Byte                   0x01
    Status Should Be            frlvl=1
    Send Byte                   0x02
    Status Should Be            rxne=1    frlvl=2
    Word Should Arrive          0x0302
    Send Byte                   0x03
    Status Should Be            frlvl=1
    Write Register              ${CR2}    ${EIGHT_BIT}
    Status Should Be            rxne=1    frlvl=1
    Byte Should Arrive          0x04

Packed Transfers Should Keep Two Half-Words In Flight
    Start                       ${HALF_WORD_RXNE}
    Send Word                   0x0201
    Send Word                   0x0403
    Word Should Arrive          0x0302
    Send Word                   0x0605
    Word Should Arrive          0x0504
    Send Word                   0x0807
    Word Should Arrive          0x0706
    Word Should Arrive          0x0908
    Write Register              ${CR2}    ${EIGHT_BIT}
    Send Byte                   0x09
    Byte Should Arrive          0x0A
    Sent Should Be              01 02 03 04 05 06 07 08 09
    Status Should Be

Fifo Level Should Count Unread Bytes Up To An Overrun
    Start
    Send Byte                   0x01
    Status Should Be            rxne=1    frlvl=1
    Send Byte                   0x02
    Status Should Be            rxne=1    frlvl=2
    Send Byte                   0x03
    Status Should Be            rxne=1    frlvl=3
    Send Byte                   0x04
    Status Should Be            rxne=1    frlvl=3
    Send Byte                   0x05
    Sent Should Be              01 02 03 04 05
    Status Should Be            rxne=1    ovr=1    frlvl=3
    Word Should Arrive          0x0302
    Word Should Arrive          0x0504

Overrun Should Clear Only On A Data Then Status Read
    Start                       ${{${EIGHT_BIT} | ${ERRIE}}}
    FOR    ${byte}    IN RANGE    1    6
        Send Byte               ${byte}
    END
    Irq Should Be               True
    Read Register               ${SR}
    Irq Should Be               True
    Byte Should Arrive          0x02
    Irq Should Be               True
    Execute Command             sysbus WriteWord ${{${SPI2} + ${SR}}} 0x0000
    Irq Should Be               True
    Send Byte                   0x06
    Status Should Be            rxne=1    ovr=1    frlvl=3
    Irq Should Be               False
    Status Should Be            rxne=1    frlvl=3
    Word Should Arrive          0x0403
    Byte Should Arrive          0x05
    Status Should Be

A Sixteen-Bit Frame Should Not Split Across A Full Fifo
    Start
    Send Byte                   0x01
    Send Byte                   0x02
    Send Byte                   0x03
    Write Register              ${CR2}    ${SIXTEEN_BIT}
    Send Word                   0x0405
    Sent Should Be              01 02 03 04 05
    Status Should Be            rxne=1    ovr=1    frlvl=3

Rxne Should Not Depend On The Frame Size
    Start
    Send Byte                   0x01
    Write Register              ${CR2}    ${SIXTEEN_BIT}
    Status Should Be            rxne=1    frlvl=1

Sixteen-Bit Frames Should Go Most Significant Byte First
    Start                       ${SIXTEEN_BIT}
    Send Word                   0xABCD
    Sent Should Be              AB CD
    Status Should Be            rxne=1    frlvl=2
    Word Should Arrive          0xACCE
    Write Register              ${CR1}    ${{${MASTER} | ${ONE_LINE}}}
    Send Word                   0x1234
    Sent Should Be              AB CD 12 34
    Status Should Be

One-Line Transmit Should Receive Nothing
    Start                       ${EIGHT_BIT}    ${{${MASTER} | ${ONE_LINE}}}
    Send Byte                   0x01
    Send Word                   0x0302
    Send Byte                   0x04
    Send Word                   0x0605
    Sent Should Be              01 02 03 04 05 06
    Status Should Be
    Write Register              ${CR1}    ${MASTER}
    Send Byte                   0x07
    Byte Should Arrive          0x08
    Status Should Be

Writes While Disabled Should Wait For The Enable
    Start                       ${{${EIGHT_BIT} | ${TXEIE}}}    0x0304
    Send Byte                   0x01
    Send Word                   0x0302
    Sent Should Be              ${EMPTY}
    Status Should Be            txe=0    ftlvl=3
    Irq Should Be               False
    Write Register              ${CR1}    ${MASTER}
    Sent Should Be              01 02 03
    Status Should Be            rxne=1    frlvl=3
    Irq Should Be               True

Rxne Interrupt Should Follow The Fifo
    Start                       ${{${EIGHT_BIT} | ${RXNEIE}}}
    Irq Should Be               False
    Send Byte                   0x01
    Irq Should Be               True
    Byte Should Arrive          0x02
    Irq Should Be               False

Receive Dma Should Take What Is Waiting
    Start
    Send Byte                   0x20
    Send Byte                   0x21
    Write Dma Register          ${CPAR4}    ${{${SPI2} + ${DR}}}
    Write Dma Register          ${CMAR4}    ${BUFFER}
    Write Dma Register          ${CNDTR4}    4
    Write Dma Register          ${CCR4}    0x0081
    Dma Should Have Left        4
    Write Register              ${CR2}    ${{${EIGHT_BIT} | ${RXDMAEN}}}
    Dma Should Have Left        2
    Buffer Should Hold          0x21    0x22
    Status Should Be
    Write Register              ${CR2}    ${{${HALF_WORD_RXNE} | ${RXDMAEN}}}
    Send Byte                   0x30
    Dma Should Have Left        2
    Write Register              ${CR2}    ${{${EIGHT_BIT} | ${RXDMAEN}}}
    Dma Should Have Left        1
    Buffer Should Hold          0x21    0x22    0x31
    Send Byte                   0x40
    Dma Should Have Left        0
    Buffer Should Hold          0x21    0x22    0x31    0x41
    Status Should Be

Interrupts Should Reach The Cpu
    [Setup]                     Create Spi Machine    spi_irq
    Word Symbol Should Be       ENTRIES    0    1
    Word Symbol Should Be       ENTRIES    1    1

Dma Should Carry Frames Both Ways
    [Setup]                     Create Spi Machine    spi_dma
    Word Symbol Should Be       LEFT    0    0
    Word Symbol Should Be       LEFT    1    0
    Sent Should Be              10 11 12 13
    FOR    ${index}    IN RANGE    4
        Byte Symbol Should Be    RX    ${{4 + ${index}}}    ${{0x11 + ${index}}}
    END

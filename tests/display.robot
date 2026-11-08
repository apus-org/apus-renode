*** Settings ***
Resource            apus.resource
Test Setup          Start Display

*** Variables ***
${PARAMETER}        0x100
${RESET_LOW}        0x200
${RESET_HIGH}       0x1000
${DESELECT}         0x400
${SELECT}           0x800
${ATTEMPTS}         100

*** Keywords ***
Send Words
    [Arguments]                 @{words}
    ${base}=                    Symbol Address    WORDS
    FOR    ${index}    ${word}    IN ENUMERATE    @{words}
        Execute Command         sysbus WriteWord ${{int($base, 0) + 2 * $index}} ${word}
    END
    ${len}=                     Get Length    ${words}
    Store Symbol                LEN    ${len}
    ${request}=                 Read Symbol    REQUEST
    ${request}=                 Evaluate    $request + 1
    Store Symbol                REQUEST    ${request}
    FOR    ${attempt}    IN RANGE    ${ATTEMPTS}
        Execute Command         emulation RunFor "0.01"
        ${done}=                Read Symbol    DONE
        IF    ${done} == ${request}    BREAK
    END
    Should Be Equal As Integers    ${done}    ${request}    bus words not sent

Send
    [Arguments]                 ${command}    @{parameters}
    ${words}=                   Evaluate    [int($command, 0)] + [int(p, 0) | ${PARAMETER} for p in $parameters]
    Send Words                  @{words}

Write Pixels
    [Arguments]                 @{pixels}
    ${bytes}=                   Evaluate    [b for p in $pixels for b in (hex(int(p, 0) >> 8), hex(int(p, 0) & 0xFF))]
    Send                        0x2C    @{bytes}

Window
    [Arguments]                 ${x0}    ${y0}    ${x1}    ${y1}
    Send                        0x2A    ${{hex(int($x0) >> 8)}}    ${{hex(int($x0) & 0xFF)}}    ${{hex(int($x1) >> 8)}}    ${{hex(int($x1) & 0xFF)}}
    Send                        0x2B    ${{hex(int($y0) >> 8)}}    ${{hex(int($y0) & 0xFF)}}    ${{hex(int($y1) >> 8)}}    ${{hex(int($y1) & 0xFF)}}

Pixel Should Be
    [Arguments]                 ${x}    ${y}    ${expected}
    ${value}=                   Execute Command    sysbus.gpioPortE.display Pixel ${x} ${y}
    Should Be Equal As Integers    ${value.strip()}    ${expected}    pixel (${x}, ${y})

Screen Should Start With
    [Arguments]                 @{rgba}
    Execute Command             emulation RunFor "0.1"
    ${count}=                   Get Length    ${rgba}
    ${shown}=                   Execute Command    python "print(list(self.Machine['sysbus.gpioPortE.display'].TakeScreenshot().Bytes[:${count}]))"
    ${expected}=                Evaluate    [int(byte) for byte in $rgba]
    Should Be Equal             ${{eval($shown.strip())}}    ${expected}

Start Display
    Create Machine              display
    Send Words                  ${RESET_LOW}    ${RESET_HIGH}
    Send                        0x11
    Send                        0x3A    0x55
    Send                        0x29

*** Test Cases ***
Window Should Show The Written Pixels
    Window                      1    1    2    2
    Write Pixels                0xA53C    0x5AC3    0x0F0F    0xF0F0
    Pixel Should Be             1    1    0xA53C
    Pixel Should Be             2    1    0x5AC3
    Pixel Should Be             1    2    0x0F0F
    Pixel Should Be             2    2    0xF0F0
    Pixel Should Be             0    1    0
    Pixel Should Be             3    1    0
    Pixel Should Be             1    0    0
    Pixel Should Be             1    3    0

Reset Pulse Should Restore The Defaults
    Write Pixels                0x1234
    Send                        0x36    0x40
    Send Words                  ${RESET_LOW}    ${RESET_HIGH}
    Pixel Should Be             0    0    0
    Send                        0x11
    Send                        0x29
    Pixel Should Be             0    0    0x1234
    Write Pixels                0x5678
    Pixel Should Be             0    0    0x1234
    Send                        0x3A    0x55
    Write Pixels                0x5678
    Pixel Should Be             0    0    0x5678

Bus Should Be Ignored While In Reset
    Write Pixels                0x1234
    Send Words                  ${RESET_LOW}    0x11    0x29    ${RESET_HIGH}
    Pixel Should Be             0    0    0

Deselected Panel Should Ignore The Bus
    Send Words                  ${DESELECT}
    Write Pixels                0x1234
    Send Words                  ${SELECT}
    Pixel Should Be             0    0    0

Screen Should Show The Colours
    Write Pixels                0xF800    0x07E0
    Screen Should Start With    255    0    0    255    0    255    0    255

Window Should Wrap To Its Start
    Window                      0    0    2    1
    Write Pixels                0x1    0x2    0x3    0x4    0x5    0x6    0x7
    Pixel Should Be             0    0    0x7
    Pixel Should Be             1    0    0x2
    Pixel Should Be             2    0    0x3
    Pixel Should Be             0    1    0x4
    Pixel Should Be             2    1    0x6
    Pixel Should Be             3    0    0

Access Control Should Swap Then Mirror
    Send                        0x36    0x60
    Window                      5    10    5    10
    Write Pixels                0x1234
    Pixel Should Be             229    5    0x1234
    Send                        0x36    0x80
    Write Pixels                0x5678
    Pixel Should Be             5    309    0x5678

Exchanged Axes Should Still Fill Along The Columns
    Send                        0x36    0x20
    Window                      0    0    1    1
    Write Pixels                0x1234    0x5678
    Pixel Should Be             0    1    0x5678
    Pixel Should Be             1    0    0

Bgr Order Should Swap Red And Blue
    Send                        0x36    0x08
    Write Pixels                0xF800
    Pixel Should Be             0    0    0x001F

Inversion Should Invert The Shown Image
    Write Pixels                0x1234
    Send                        0x21
    Pixel Should Be             0    0    0xEDCB
    Send                        0x20
    Pixel Should Be             0    0    0x1234

Display Off And Sleep Should Blank The Panel
    Write Pixels                0x1234
    Send                        0x28
    Pixel Should Be             0    0    0
    Send                        0x29
    Pixel Should Be             0    0    0x1234
    Send                        0x10
    Pixel Should Be             0    0    0
    Send                        0x11
    Pixel Should Be             0    0    0x1234

Software Reset Should Keep Format And Access Control
    Send                        0x36    0x40
    Send                        0x01
    Send                        0x11
    Send                        0x29
    Write Pixels                0x1234
    Pixel Should Be             239    0    0x1234

Full Screen Fill Should Reach The Last Pixel
    Send                        0x2C
    Store Symbol                FILL_COLOUR    0x07E0
    Store Symbol                FILL    76800
    Send Words
    Pixel Should Be             0    0    0x07E0
    Pixel Should Be             239    0    0x07E0
    Pixel Should Be             0    319    0x07E0
    Pixel Should Be             239    319    0x07E0

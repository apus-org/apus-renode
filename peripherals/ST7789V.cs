using System.Collections.Generic;
using Antmicro.Renode.Backends.Display;
using Antmicro.Renode.Core;
using Antmicro.Renode.Exceptions;
using Antmicro.Renode.Logging;

namespace Antmicro.Renode.Peripherals.Video
{
    public class ST7789V : AutoRepaintingVideo, IGPIOReceiver
    {
        public ST7789V(IMachine machine) : base(machine)
        {
            Reconfigure(Columns, Rows, PixelFormat.RGB565);
            HardwareReset();
        }

        public void OnGPIO(int number, bool value)
        {
            if(number < 0 || number >= Inputs)
            {
                this.Log(LogLevel.Error, "No input {0}: 0–7 DB0–DB7, 8 nCS, 9 D/C, 10 nWR, 11 nRESET", number);
                return;
            }
            var was = pins[number];
            pins[number] = value;

            if(number == NotReset && !value)
            {
                HardwareReset();
            }
            else if(number == NotWrite && !was && value && pins[NotReset] && !pins[NotChipSelect])
            {
                Latch();
            }
        }

        public override void Reset()
        {
        }

        public int Pixel(int x, int y)
        {
            if(x < 0 || x >= Columns || y < 0 || y >= Rows)
            {
                throw new RecoverableException($"({x}, {y}) is outside the {Columns}x{Rows} panel");
            }
            return Shown(gram[y * Columns + x]);
        }

        protected override void Repaint()
        {
            for(var i = 0; i < gram.Length; i++)
            {
                var value = Shown(gram[i]);
                buffer[2 * i] = (byte)value;
                buffer[2 * i + 1] = (byte)(value >> 8);
            }
        }

        private int Shown(ushort value)
        {
            if(sleeping || !displayOn)
            {
                return 0;
            }
            return inverted ? value ^ 0xFFFF : value;
        }

        private void Latch()
        {
            byte data = 0;
            for(var i = 0; i < 8; i++)
            {
                data |= (byte)(pins[i] ? 1 << i : 0);
            }

            if(pins[DataCommand])
            {
                Parameter(data);
            }
            else
            {
                Command(data);
            }
        }

        private void Command(byte opcode)
        {
            command = opcode;
            parameter = 0;
            switch(opcode)
            {
            case Nop:
            case NormalMode:
            case ColumnAddress:
            case RowAddress:
            case AccessControl:
            case PixelFormatSet:
                break;
            case SoftwareReset:
                SoftwareResetValues();
                break;
            case SleepIn:
                sleeping = true;
                break;
            case SleepOut:
                sleeping = false;
                break;
            case InversionOff:
                inverted = false;
                break;
            case InversionOn:
                inverted = true;
                break;
            case DisplayOff:
                displayOn = false;
                break;
            case DisplayOn:
                displayOn = true;
                break;
            case MemoryWrite:
                column = columnStart;
                page = pageStart;
                break;
            default:
                if(warned.Add(opcode))
                {
                    this.Log(LogLevel.Warning, "Command 0x{0:X2} not modelled, ignored", opcode);
                }
                break;
            }
        }

        private void Parameter(byte data)
        {
            switch(command)
            {
            case ColumnAddress:
                SetRange(ref columnStart, ref columnEnd, data);
                break;
            case RowAddress:
                SetRange(ref pageStart, ref pageEnd, data);
                break;
            case AccessControl:
                accessControl = data;
                break;
            case PixelFormatSet:
                pixelFormat = data;
                break;
            case MemoryWrite:
                if((pixelFormat & 0x07) != Bits16)
                {
                    if(warned.Add(PixelFormatSet))
                    {
                        this.Log(LogLevel.Warning, "Only 16-bit pixels are modelled, COLMOD 0x{0:X2} data dropped", pixelFormat);
                    }
                    break;
                }
                if(parameter % 2 == 0)
                {
                    high = data;
                }
                else
                {
                    Write((ushort)(high << 8 | data));
                }
                break;
            }
            parameter++;
        }

        private void SetRange(ref int start, ref int end, byte data)
        {
            switch(parameter)
            {
            case 0:
                start = data << 8 | start & 0xFF;
                break;
            case 1:
                start = start & 0xFF00 | data;
                break;
            case 2:
                end = data << 8 | end & 0xFF;
                break;
            case 3:
                end = end & 0xFF00 | data;
                break;
            }
        }

        private void Write(ushort value)
        {
            if((accessControl & Bgr) != 0)
            {
                value = (ushort)((value & 0x1F) << 11 | value & 0x07E0 | value >> 11);
            }

            var swapped = (accessControl & RowColumnExchange) != 0;
            var x = swapped ? page : column;
            var y = swapped ? column : page;
            if((accessControl & ColumnOrder) != 0)
            {
                x = Columns - 1 - x;
            }
            if((accessControl & RowOrder) != 0)
            {
                y = Rows - 1 - y;
            }
            if(x >= 0 && x < Columns && y >= 0 && y < Rows)
            {
                gram[y * Columns + x] = value;
            }

            if(++column > columnEnd)
            {
                column = columnStart;
                if(++page > pageEnd)
                {
                    page = pageStart;
                }
            }
        }

        private void HardwareReset()
        {
            accessControl = 0;
            pixelFormat = Bits18;
            SoftwareResetValues();
        }

        private void SoftwareResetValues()
        {
            var swapped = (accessControl & RowColumnExchange) != 0;
            sleeping = true;
            displayOn = false;
            inverted = false;
            columnStart = 0;
            pageStart = 0;
            columnEnd = (swapped ? Rows : Columns) - 1;
            pageEnd = (swapped ? Columns : Rows) - 1;
            command = Nop;
        }

        private byte command;
        private int parameter;
        private byte high;
        private byte accessControl;
        private byte pixelFormat;
        private bool sleeping;
        private bool displayOn;
        private bool inverted;
        private int columnStart;
        private int columnEnd;
        private int pageStart;
        private int pageEnd;
        private int column;
        private int page;

        private readonly bool[] pins = new bool[Inputs];
        private readonly ushort[] gram = new ushort[Columns * Rows];
        private readonly HashSet<byte> warned = new HashSet<byte>();

        private const int Columns = 240;
        private const int Rows = 320;
        private const int Inputs = 12;
        private const int NotChipSelect = 8;
        private const int DataCommand = 9;
        private const int NotWrite = 10;
        private const int NotReset = 11;

        private const byte Nop = 0x00;
        private const byte SoftwareReset = 0x01;
        private const byte SleepIn = 0x10;
        private const byte SleepOut = 0x11;
        private const byte NormalMode = 0x13;
        private const byte InversionOff = 0x20;
        private const byte InversionOn = 0x21;
        private const byte DisplayOff = 0x28;
        private const byte DisplayOn = 0x29;
        private const byte ColumnAddress = 0x2A;
        private const byte RowAddress = 0x2B;
        private const byte MemoryWrite = 0x2C;
        private const byte AccessControl = 0x36;
        private const byte PixelFormatSet = 0x3A;

        private const byte RowOrder = 0x80;
        private const byte ColumnOrder = 0x40;
        private const byte RowColumnExchange = 0x20;
        private const byte Bgr = 0x08;
        private const byte Bits16 = 0x05;
        private const byte Bits18 = 0x06;
    }
}

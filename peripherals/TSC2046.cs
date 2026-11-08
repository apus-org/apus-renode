using System;
using System.Collections.Generic;
using Antmicro.Renode.Core;
using Antmicro.Renode.Logging;
using Antmicro.Renode.Peripherals.SPI;

namespace Antmicro.Renode.Peripherals.Input
{
    public class TSC2046 : ISPIPeripheral, IGPIOReceiver, IAbsolutePositionPointerInput
    {
        public TSC2046(IMachine machine, int width, int height, int rawLeft, int rawRight, int rawTop, int rawBottom)
        {
            this.width = width;
            this.height = height;
            this.rawLeft = rawLeft;
            this.rawRight = rawRight;
            this.rawTop = rawTop;
            this.rawBottom = rawBottom;
            PenIrq = new GPIO();
            if(machine is Machine resettable)
            {
                resettable.MachineReset += _ => Drive(true);
            }
            penEnabled = true;
            Drive(false);
        }

        public byte Transmit(byte data)
        {
            if(deselected)
            {
                return 0;
            }

            byte output = 0;
            if(pending.Count > 0)
            {
                output = pending.Dequeue();
                if(pending.Count == 0)
                {
                    EndConversion();
                }
            }
            if((data & Start) != 0 && pending.Count == 0)
            {
                StartConversion(data);
            }
            return output;
        }

        public void FinishTransmission()
        {
            Abort();
        }

        public void OnGPIO(int number, bool value)
        {
            if(number != 0)
            {
                this.Log(LogLevel.Error, "No input {0}: 0 is nCS", number);
                return;
            }
            deselected = value;
            if(value)
            {
                Abort();
            }
        }

        public void Reset()
        {
        }

        public void MoveTo(int x, int y)
        {
            this.x = Math.Min(Math.Max(x, MinX), MaxX);
            this.y = Math.Min(Math.Max(y, MinY), MaxY);
        }

        public void Press(MouseButton button = MouseButton.Left)
        {
            pressed = true;
            Drive(false);
        }

        public void Release(MouseButton button = MouseButton.Left)
        {
            pressed = false;
            Drive(false);
        }

        public int MaxX => width - 1;
        public int MaxY => height - 1;
        public int MinX => 0;
        public int MinY => 0;

        public GPIO PenIrq { get; }

        private static int Interpolate(int position, int last, int first, int end)
        {
            return (int)Math.Round(first + (double)position * (end - first) / last);
        }

        private void StartConversion(byte command)
        {
            var channel = (command >> 4) & 0x07;
            int value;
            if((command & EightBit) != 0)
            {
                Warn("8-bit conversions");
                value = 0;
            }
            else
            {
                switch(channel)
                {
                case X:
                    value = pressed ? Interpolate(x, MaxX, rawLeft, rawRight) : 0;
                    break;
                case Y:
                    value = pressed ? Interpolate(y, MaxY, rawTop, rawBottom) : 0;
                    break;
                case Z1:
                    value = pressed ? PressedZ1 : 0;
                    break;
                case Z2:
                    value = pressed ? PressedZ2 : FullScale;
                    break;
                default:
                    Warn("temperature, battery and auxiliary inputs");
                    value = 0;
                    break;
                }
            }

            pending.Clear();
            pending.Enqueue((byte)(value >> 5));
            pending.Enqueue((byte)(value << 3 & 0xF8));
            converting = true;
            touchChannel = channel == X || channel == Y || channel == Z1 || channel == Z2;
            powerDownEnabled = (command & PowerDown0) == 0;
            Drive(false);
        }

        private void EndConversion()
        {
            converting = false;
            penEnabled = powerDownEnabled;
            Drive(false);
        }

        private void Abort()
        {
            pending.Clear();
            if(converting)
            {
                EndConversion();
            }
        }

        private void Warn(string what)
        {
            if(!warned)
            {
                this.Log(LogLevel.Warning, "Only 12-bit touch conversions are modelled, {0} read 0", what);
                warned = true;
            }
        }

        private void Drive(bool afterReset)
        {
            bool level;
            if(converting)
            {
                level = !touchChannel;
            }
            else
            {
                level = !(penEnabled && pressed);
            }
            if(afterReset)
            {
                PenIrq.Unset();
            }
            PenIrq.Set(level);
        }

        private int x;
        private int y;
        private bool pressed;
        private bool deselected;
        private bool converting;
        private bool touchChannel;
        private bool powerDownEnabled;
        private bool penEnabled;
        private bool warned;

        private readonly Queue<byte> pending = new Queue<byte>();
        private readonly int width;
        private readonly int height;
        private readonly int rawLeft;
        private readonly int rawRight;
        private readonly int rawTop;
        private readonly int rawBottom;

        private const byte Start = 0x80;
        private const byte EightBit = 0x08;
        private const byte PowerDown0 = 0x01;
        private const int X = 5;
        private const int Y = 1;
        private const int Z1 = 3;
        private const int Z2 = 4;
        private const int PressedZ1 = 2048;
        private const int PressedZ2 = 2560;
        private const int FullScale = 4095;
    }
}

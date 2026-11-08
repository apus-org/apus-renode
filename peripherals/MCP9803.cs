using System;
using Antmicro.Renode.Core;
using Antmicro.Renode.Logging;
using Antmicro.Renode.Peripherals.I2C;
using Antmicro.Renode.Peripherals.Sensor;
using Antmicro.Renode.Peripherals.Timers;

namespace Antmicro.Renode.Peripherals.Sensors
{
    public class MCP9803 : II2CPeripheral, ITemperatureSensor
    {
        public MCP9803(IMachine machine)
        {
            Alert = new GPIO();
            conversion = new LimitTimer(machine.ClockSource, 1000, this, "conversion", eventEnabled: true);
            conversion.LimitReached += Convert;
            if(machine is Machine resettable)
            {
                resettable.MachineReset += _ => Drive(true);
            }

            configuration = 0;
            hysteresis = 0x4B00;
            limit = 0x5000;
            temperature = 25;
            above = true;
            Run();
            Drive(false);
        }

        public void Write(byte[] data)
        {
            if(data.Length == 0)
            {
                return;
            }
            if((data[0] & ~PointerMask) != 0)
            {
                this.Log(LogLevel.Warning, "Pointer 0x{0:X2} has bits other than P1:P0 set", data[0]);
            }
            pointer = (Register)(data[0] & PointerMask);
            if(data.Length == 1)
            {
                return;
            }

            switch(pointer)
            {
            case Register.Temperature:
                this.Log(LogLevel.Warning, "TA is read-only");
                break;
            case Register.Configuration:
                Configure(data[data.Length - 1]);
                break;
            default:
                var value = (ushort)((data[1] << 8 | (data.Length > 2 ? data[2] : 0)) & LimitMask);
                if(pointer == Register.Hysteresis)
                {
                    hysteresis = value;
                }
                else
                {
                    limit = value;
                }
                break;
            }
        }

        public byte[] Read(int count = 1)
        {
            var bytes = pointer == Register.Configuration
                ? new[] { (byte)(configuration | (oneShot ? OneShot : 0)) }
                : Split(pointer == Register.Temperature ? ambient : pointer == Register.Hysteresis ? hysteresis : limit);

            var result = new byte[count];
            for(var i = 0; i < count; i++)
            {
                result[i] = bytes[i % bytes.Length];
            }

            if(InterruptMode && asserted)
            {
                asserted = false;
                Drive(false);
            }
            return result;
        }

        public void FinishTransmission()
        {
        }

        public void Reset()
        {
        }

        public decimal Temperature
        {
            get => temperature;
            set
            {
                if(value < MinimumTemperature || value > MaximumTemperature)
                {
                    this.Log(LogLevel.Warning, "{0} °C clamped to {1}..{2} °C", value, MinimumTemperature, MaximumTemperature);
                }
                temperature = Math.Min(Math.Max(value, MinimumTemperature), MaximumTemperature);
            }
        }

        public GPIO Alert { get; }

        private static byte[] Split(ushort value) => new[] { (byte)(value >> 8), (byte)value };

        private void Configure(byte value)
        {
            var wasShutdown = Shutdown;
            var resolution = Resolution;
            configuration = (byte)(value & ~OneShot);

            if(Shutdown)
            {
                if(!wasShutdown && InterruptMode)
                {
                    asserted = false;
                }
                if(wasShutdown && (value & OneShot) != 0)
                {
                    oneShot = true;
                    Run();
                }
                else if(!oneShot)
                {
                    conversion.Enabled = false;
                }
            }
            else if(wasShutdown || resolution != Resolution)
            {
                oneShot = false;
                Run();
            }
            Drive(false);
        }

        private void Run()
        {
            conversion.Enabled = false;
            conversion.Limit = ConversionMilliseconds[Resolution];
            conversion.ResetValue();
            conversion.Enabled = true;
        }

        private void Convert()
        {
            var code = (int)Math.Floor(temperature * 16);
            var dropped = 3 - Resolution;
            ambient = (ushort)(code >> dropped << dropped << 4);

            if(oneShot)
            {
                oneShot = false;
                conversion.Enabled = false;
            }

            var value = (short)ambient;
            bool fault;
            if(InterruptMode)
            {
                fault = above ? value > (short)limit : value < (short)hysteresis;
            }
            else
            {
                fault = asserted ? value < (short)hysteresis : value > (short)limit;
            }

            faults = fault ? faults + 1 : 0;
            if(faults < FaultQueue[(configuration >> 3) & 3])
            {
                return;
            }
            faults = 0;
            if(InterruptMode)
            {
                asserted = true;
                above = !above;
            }
            else
            {
                asserted = !asserted;
            }
            Drive(false);
        }

        private void Drive(bool afterReset)
        {
            var level = asserted == ((configuration & ActiveHigh) != 0);
            if(afterReset)
            {
                Alert.Unset();
            }
            Alert.Set(level);
        }

        private bool Shutdown => (configuration & ShutdownBit) != 0;
        private bool InterruptMode => (configuration & InterruptBit) != 0;
        private int Resolution => (configuration >> 5) & 3;

        private Register pointer;
        private byte configuration;
        private ushort ambient;
        private ushort hysteresis;
        private ushort limit;
        private decimal temperature;
        private bool asserted;
        private bool above;
        private bool oneShot;
        private int faults;

        private readonly LimitTimer conversion;

        private const int PointerMask = 0x03;
        private const int LimitMask = 0xFF80;
        private const byte ShutdownBit = 0x01;
        private const byte InterruptBit = 0x02;
        private const byte ActiveHigh = 0x04;
        private const byte OneShot = 0x80;
        private const decimal MinimumTemperature = -55;
        private const decimal MaximumTemperature = 125;

        private static readonly ulong[] ConversionMilliseconds = { 30, 60, 120, 240 };
        private static readonly int[] FaultQueue = { 1, 2, 4, 6 };

        private enum Register
        {
            Temperature,
            Configuration,
            Hysteresis,
            Limit,
        }
    }
}

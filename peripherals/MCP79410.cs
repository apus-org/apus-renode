using System;
using Antmicro.Renode.Core;
using Antmicro.Renode.Logging;
using Antmicro.Renode.Peripherals.I2C;

namespace Antmicro.Renode.Peripherals.Timers
{
    public class MCP79410 : II2CPeripheral
    {
        public MCP79410(IMachine machine)
        {
            MFP = new GPIO();
            oscillator = new LimitTimer(machine.ClockSource, OscillatorHz, this, "oscillator", limit: OscillatorHz, eventEnabled: true);
            oscillator.LimitReached += Tick;
            if(machine is Machine resettable)
            {
                resettable.MachineReset += _ => Drive(true);
            }

            registers[Weekday] = 0x01;
            registers[Date] = 0x01;
            registers[Month] = 0x01;
            registers[Year] = 0x01;
            registers[Control] = Out;
            foreach(var alarm in Alarms)
            {
                registers[alarm + 3] = 0x01;
                registers[alarm + 4] = 0x01;
                registers[alarm + 5] = 0x01;
            }
            Drive(false);
        }

        public void Write(byte[] data)
        {
            if(data.Length == 0)
            {
                return;
            }
            Pointer = data[0];
            for(var i = 1; i < data.Length; i++)
            {
                if(Pointer >= SramEnd)
                {
                    this.Log(LogLevel.Warning, "Write to invalid address 0x{0:X2} dropped", Pointer);
                    return;
                }
                if(Pointer == Unlock)
                {
                    unlockStep = data[i] == 0x55 ? 1 : data[i] == 0xAA && unlock == 1 ? 2 : 0;
                }
                if(Pointer < SramStart)
                {
                    WriteRegister(Pointer, data[i]);
                }
                else
                {
                    sram[Pointer - SramStart] = data[i];
                }
                Pointer = Next(Pointer);
            }
            Update();
        }

        public byte[] Read(int count = 1)
        {
            var snapshot = Snapshot();
            var result = new byte[count];
            for(var i = 0; i < count; i++)
            {
                if(Pointer < SramStart)
                {
                    result[i] = snapshot[Pointer];
                }
                else if(Pointer < SramEnd)
                {
                    result[i] = sram[Pointer - SramStart];
                }
                else
                {
                    this.Log(LogLevel.Warning, "Read from invalid address 0x{0:X2}", Pointer);
                    result[i] = 0xFF;
                }
                Pointer = Next(Pointer);
                if(Pointer == 0)
                {
                    snapshot = Snapshot();
                }
            }
            return result;
        }

        public void FinishTransmission()
        {
            unlock = unlockStep;
            unlockStep = 0;
        }

        public void Reset()
        {
            unlock = 0;
            unlockStep = 0;
        }

        public GPIO MFP { get; }

        internal byte Pointer { get; set; }

        internal bool TakeUnlock()
        {
            var unlocked = unlock == 2;
            unlock = 0;
            return unlocked;
        }

        private static int FromBcd(byte value) => (value >> 4) * 10 + (value & 0x0F);

        private static byte ToBcd(int value) => (byte)((value / 10) << 4 | value % 10);

        private static byte Next(byte address)
        {
            if(address == RegistersEnd - 1)
            {
                return 0;
            }
            if(address == SramEnd - 1)
            {
                return SramStart;
            }
            return (byte)(address + 1);
        }

        private byte[] Snapshot()
        {
            var snapshot = (byte[])registers.Clone();
            snapshot[Weekday] |= (byte)(oscillator.Enabled ? OscRun : 0);
            snapshot[Month] |= (byte)(FromBcd(registers[Year]) % 4 == 0 ? LeapYear : 0);
            foreach(var alarm in Alarms)
            {
                snapshot[alarm + 2] |= (byte)(registers[Hour] & TwelveHour);
            }
            snapshot[Alarms[1] + 3] = (byte)(snapshot[Alarms[1] + 3] & ~Polarity | registers[Alarms[0] + 3] & Polarity);
            return snapshot;
        }

        private void WriteRegister(byte address, byte value)
        {
            registers[address] = (byte)(value & WriteMasks[address]);
            switch(address)
            {
            case Seconds:
                var start = (value & Start) != 0;
                if(start != oscillator.Enabled)
                {
                    oscillator.ResetValue();
                    oscillator.Enabled = start;
                }
                break;
            case Hour:
                if((value & TwelveHour) != 0)
                {
                    this.Log(LogLevel.Warning, "12-hour format is not modelled, counting as 24-hour");
                }
                break;
            case Control:
                if((value & (SquareWave | ExternalOscillator)) != 0)
                {
                    this.Log(LogLevel.Warning, "SQWEN and EXTOSC are not modelled");
                }
                break;
            }
        }

        private void Tick()
        {
            var seconds = FromBcd((byte)(registers[Seconds] & 0x7F)) + 1;
            registers[Seconds] = (byte)(registers[Seconds] & Start | ToBcd(seconds % 60));
            if(seconds >= 60)
            {
                var minutes = FromBcd(registers[Minutes]) + 1;
                registers[Minutes] = ToBcd(minutes % 60);
                if(minutes >= 60)
                {
                    var hours = FromBcd((byte)(registers[Hour] & 0x3F)) + 1;
                    registers[Hour] = (byte)(registers[Hour] & TwelveHour | ToBcd(hours % 24));
                    if(hours >= 24)
                    {
                        NextDay();
                    }
                }
            }
            Update();
        }

        private void NextDay()
        {
            var weekday = registers[Weekday] & 0x07;
            registers[Weekday] = (byte)(registers[Weekday] & ~0x07 | weekday % 7 + 1);

            var year = FromBcd(registers[Year]);
            var month = FromBcd(registers[Month]);
            var date = FromBcd(registers[Date]) + 1;
            if(date > DaysInMonth(month, year))
            {
                date = 1;
                month++;
                if(month > 12)
                {
                    month = 1;
                    registers[Year] = ToBcd((year + 1) % 100);
                }
                registers[Month] = ToBcd(month);
            }
            registers[Date] = ToBcd(date);
        }

        private static int DaysInMonth(int month, int year)
        {
            switch(month)
            {
            case 2:
                return year % 4 == 0 ? 29 : 28;
            case 4:
            case 6:
            case 9:
            case 11:
                return 30;
            default:
                return 31;
            }
        }

        private void Update()
        {
            for(var i = 0; i < Alarms.Length; i++)
            {
                if((registers[Control] & AlarmEnable[i]) != 0 && Matches(Alarms[i]))
                {
                    registers[Alarms[i] + 3] |= AlarmFlag;
                }
            }
            Drive(false);
        }

        private bool Matches(int alarm)
        {
            bool Same(int offset, int register, int mask) => (registers[alarm + offset] & mask) == (registers[register] & mask);

            switch((registers[alarm + 3] >> 4) & 0x07)
            {
            case 0:
                return Same(0, Seconds, 0x7F);
            case 1:
                return Same(1, Minutes, 0x7F);
            case 2:
                return Same(2, Hour, 0x3F);
            case 3:
                return Same(3, Weekday, 0x07);
            case 4:
                return Same(4, Date, 0x3F);
            case 7:
                return Same(0, Seconds, 0x7F) && Same(1, Minutes, 0x7F) && Same(2, Hour, 0x3F)
                    && Same(3, Weekday, 0x07) && Same(4, Date, 0x3F) && Same(5, Month, 0x1F);
            default:
                return false;
            }
        }

        private bool Level()
        {
            var control = registers[Control];
            if((control & SquareWave) != 0)
            {
                return true;
            }

            var polarity = (registers[Alarms[0] + 3] & Polarity) != 0;
            var flags = new bool[Alarms.Length];
            var enabled = new bool[Alarms.Length];
            for(var i = 0; i < Alarms.Length; i++)
            {
                flags[i] = (registers[Alarms[i] + 3] & AlarmFlag) != 0;
                enabled[i] = (control & AlarmEnable[i]) != 0;
            }

            if(enabled[0] && enabled[1])
            {
                return polarity ? flags[0] || flags[1] : !(flags[0] && flags[1]);
            }
            for(var i = 0; i < Alarms.Length; i++)
            {
                if(enabled[i])
                {
                    return flags[i] == polarity;
                }
            }
            return (control & Out) != 0;
        }

        private void Drive(bool afterReset)
        {
            if(afterReset)
            {
                MFP.Unset();
            }
            MFP.Set(Level());
        }

        private int unlock;
        private int unlockStep;

        private readonly byte[] registers = new byte[RegistersEnd];
        private readonly byte[] sram = new byte[SramEnd - SramStart];
        private readonly LimitTimer oscillator;

        private const ulong OscillatorHz = 32768;
        private const byte Seconds = 0x00;
        private const byte Minutes = 0x01;
        private const byte Hour = 0x02;
        private const byte Weekday = 0x03;
        private const byte Date = 0x04;
        private const byte Month = 0x05;
        private const byte Year = 0x06;
        private const byte Control = 0x07;
        private const byte Unlock = 0x09;
        private const byte RegistersEnd = 0x20;
        private const byte SramStart = 0x20;
        private const byte SramEnd = 0x60;
        private const byte Start = 0x80;
        private const byte TwelveHour = 0x40;
        private const byte OscRun = 0x20;
        private const byte LeapYear = 0x20;
        private const byte Out = 0x80;
        private const byte SquareWave = 0x40;
        private const byte ExternalOscillator = 0x08;
        private const byte Polarity = 0x80;
        private const byte AlarmFlag = 0x08;

        private static readonly byte[] Alarms = { 0x0A, 0x11 };
        private static readonly byte[] AlarmEnable = { 0x10, 0x20 };
        private static readonly byte[] WriteMasks =
        {
            0xFF, 0x7F, 0x7F, 0x0F, 0x3F, 0x1F, 0xFF, 0xFF,
            0xFF, 0x00, 0x7F, 0x7F, 0x3F, 0xF7, 0x3F, 0x1F,
            0xFF, 0x7F, 0x7F, 0x3F, 0x77, 0x3F, 0x1F, 0xFF,
            0x7F, 0x7F, 0x3F, 0xFF, 0x7F, 0x7F, 0x3F, 0xFF,
        };
    }
}

namespace Antmicro.Renode.Peripherals.Memory
{
    public class MCP79410_EEPROM : II2CPeripheral
    {
        public MCP79410_EEPROM(Timers.MCP79410 rtc)
        {
            this.rtc = rtc;
            for(var i = 0; i < array.Length; i++)
            {
                array[i] = 0xFF;
            }
            for(var i = 0; i < protectedBlock.Length; i++)
            {
                protectedBlock[i] = 0xFF;
            }
        }

        public void Write(byte[] data)
        {
            if(data.Length == 0)
            {
                return;
            }
            var address = data[0];
            for(var i = 1; i < data.Length; i++)
            {
                if(address == Status)
                {
                    pendingStatus = data[i];
                    continue;
                }
                if(!IsArray(address) && !IsProtected(address))
                {
                    this.Log(LogLevel.Warning, "Write to invalid address 0x{0:X2} dropped", address);
                    break;
                }
                pending[address & PageMask] = data[i];
                pendingAddress[address & PageMask] = address;
                address = (byte)(address & ~PageMask | (address + 1) & PageMask);
            }
            rtc.Pointer = address;
        }

        public byte[] Read(int count = 1)
        {
            var result = new byte[count];
            for(var i = 0; i < count; i++)
            {
                var address = rtc.Pointer;
                if(IsArray(address))
                {
                    result[i] = array[address];
                    address = (byte)((address + 1) % ArraySize);
                }
                else if(IsProtected(address))
                {
                    result[i] = protectedBlock[address - ProtectedStart];
                    address++;
                }
                else if(address == Status)
                {
                    result[i] = status;
                    address++;
                }
                else
                {
                    this.Log(LogLevel.Warning, "Read from invalid address 0x{0:X2}", address);
                    result[i] = 0xFF;
                    address++;
                }
                rtc.Pointer = address;
            }
            return result;
        }

        public void FinishTransmission()
        {
            var unlocked = rtc.TakeUnlock();
            if(pendingStatus.HasValue)
            {
                status = (byte)(pendingStatus.Value & BlockProtectMask);
            }
            for(var i = 0; i < pending.Length; i++)
            {
                if(!pending[i].HasValue)
                {
                    continue;
                }
                var address = pendingAddress[i];
                if(IsProtected(address))
                {
                    if(unlocked)
                    {
                        protectedBlock[address - ProtectedStart] = pending[i].Value;
                    }
                    else
                    {
                        this.Log(LogLevel.Warning, "Protected EEPROM write to 0x{0:X2} without unlock dropped", address);
                    }
                }
                else if(address < ProtectedFrom[(status >> 2) & 3])
                {
                    array[address] = pending[i].Value;
                }
                else
                {
                    this.Log(LogLevel.Warning, "Write to block-protected address 0x{0:X2} dropped", address);
                }
            }
            Reset();
        }

        public void Reset()
        {
            pendingStatus = null;
            for(var i = 0; i < pending.Length; i++)
            {
                pending[i] = null;
            }
        }

        private static bool IsArray(byte address) => address < ArraySize;

        private static bool IsProtected(byte address) => address >= ProtectedStart && address < ProtectedStart + PageSize;

        private byte status;
        private byte? pendingStatus;

        private readonly Timers.MCP79410 rtc;
        private readonly byte[] array = new byte[ArraySize];
        private readonly byte[] protectedBlock = new byte[PageSize];
        private readonly byte?[] pending = new byte?[PageSize];
        private readonly byte[] pendingAddress = new byte[PageSize];

        private const int ArraySize = 0x80;
        private const int PageSize = 8;
        private const int PageMask = PageSize - 1;
        private const byte ProtectedStart = 0xF0;
        private const byte Status = 0xFF;
        private const byte BlockProtectMask = 0x0C;

        private static readonly int[] ProtectedFrom = { ArraySize, 0x60, 0x40, 0x00 };
    }
}

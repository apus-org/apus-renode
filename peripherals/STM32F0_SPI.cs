using System.Collections.Generic;
using Antmicro.Renode.Core;
using Antmicro.Renode.Core.Structure;
using Antmicro.Renode.Core.Structure.Registers;
using Antmicro.Renode.Logging;
using Antmicro.Renode.Peripherals.Bus;

namespace Antmicro.Renode.Peripherals.SPI
{
    public class STM32F0_SPI : NullRegistrationPointPeripheralContainer<ISPIPeripheral>, IBytePeripheral, IWordPeripheral, IDoubleWordPeripheral, IKnownSize
    {
        public STM32F0_SPI(IMachine machine) : base(machine)
        {
            IRQ = new GPIO();
            DMAReceive = new GPIO();
            registers = new DoubleWordRegisterCollection(this);
            DefineRegisters();
            Reset();
        }

        public override void Reset()
        {
            registers.Reset();
            receiveFifo.Clear();
            transmitFifo.Clear();
            overrun = false;
            dataReadInOverrun = false;
            warnedUnconnected = false;
            IRQ.Unset();
            DMAReceive.Unset();
        }

        public byte ReadByte(long offset)
        {
            if(offset == DataRegister)
            {
                return (byte)ReadData(1);
            }
            return (byte)(registers.Read(offset & ~3) >> (int)(8 * (offset & 3)));
        }

        public void WriteByte(long offset, byte value)
        {
            if(offset == DataRegister)
            {
                WriteData(value, 1);
                return;
            }
            WritePart(offset, value, 0xFF);
        }

        public ushort ReadWord(long offset)
        {
            if(offset == DataRegister)
            {
                return (ushort)ReadData(2);
            }
            return (ushort)(registers.Read(offset & ~3) >> (int)(8 * (offset & 2)));
        }

        public void WriteWord(long offset, ushort value)
        {
            if(offset == DataRegister)
            {
                WriteData(value, 2);
                return;
            }
            WritePart(offset & ~1, value, 0xFFFF);
        }

        public uint ReadDoubleWord(long offset)
        {
            return offset == DataRegister ? ReadData(2) : registers.Read(offset);
        }

        public void WriteDoubleWord(long offset, uint value)
        {
            if(offset == DataRegister)
            {
                WriteData(value, 2);
                return;
            }
            registers.Write(offset, value);
        }

        public long Size => 0x400;

        public GPIO IRQ { get; }
        public GPIO DMAReceive { get; }

        private void WritePart(long offset, uint value, uint mask)
        {
            var shift = (int)(8 * (offset & 3));
            var aligned = offset & ~3;
            if(aligned == (long)Registers.Status)
            {
                return;
            }
            var old = registers.Read(aligned);
            registers.Write(aligned, old & ~(mask << shift) | (value & mask) << shift);
        }

        private bool SixteenBitFrames => dataSize.Value == SixteenBits;

        private bool SendOnly => bidirectionalMode.Value && bidirectionalOutput.Value;

        private bool TransmitEmpty => transmitFifo.Count <= FifoBytes / 2;

        private bool ReceiveNotEmpty => receiveFifo.Count >= (receiveThreshold.Value ? 1 : 2);

        private static int Level(int bytes) => bytes >= 3 ? 3 : bytes;

        private void WriteData(uint value, int width)
        {
            if(SixteenBitFrames)
            {
                if(width == 1)
                {
                    this.Log(LogLevel.Warning, "8-bit DR write with 16-bit frames, sent as a frame with a zero high byte");
                }
                Queue(new[] { (byte)(value >> 8), (byte)value });
            }
            else if(width == 1)
            {
                Queue(new[] { (byte)value });
            }
            else
            {
                Queue(new[] { (byte)value });
                Queue(new[] { (byte)(value >> 8) });
            }
            if(enable.Value)
            {
                SendQueued();
            }
            Update();
        }

        private void Queue(byte[] frame)
        {
            if(transmitFifo.Count + frame.Length > FifoBytes)
            {
                this.Log(LogLevel.Warning, "TX FIFO full while the SPI is disabled, frame dropped");
                return;
            }
            transmitFifo.Enqueue(frame);
        }

        private void SendQueued()
        {
            while(transmitFifo.Count > 0)
            {
                var frame = transmitFifo.Dequeue();
                var received = new byte[frame.Length];
                for(var i = 0; i < frame.Length; i++)
                {
                    received[i] = Exchange(frame[i]);
                }
                if(!SendOnly)
                {
                    Receive(received);
                }
            }
            RequestReceiveDma();
        }

        private byte Exchange(byte data)
        {
            var peripheral = RegisteredPeripheral;
            if(peripheral == null)
            {
                if(!warnedUnconnected)
                {
                    this.Log(LogLevel.Warning, "Frame sent with no device on the bus, received 0");
                    warnedUnconnected = true;
                }
                return 0;
            }
            return peripheral.Transmit(data);
        }

        private void Receive(byte[] frame)
        {
            if(overrun)
            {
                return;
            }
            if(receiveFifo.Count + frame.Length > FifoBytes)
            {
                overrun = true;
                dataReadInOverrun = false;
                return;
            }
            // a 16-bit frame is read low byte first
            for(var i = frame.Length - 1; i >= 0; i--)
            {
                receiveFifo.Enqueue(frame[i]);
            }
        }

        private void RequestReceiveDma()
        {
            while(receiveDma.Value && ReceiveNotEmpty)
            {
                var before = receiveFifo.Count;
                DMAReceive.Blink();
                if(receiveFifo.Count == before)
                {
                    break;
                }
            }
        }

        private uint ReadData(int width)
        {
            if(overrun)
            {
                dataReadInOverrun = true;
            }
            uint value = Dequeue();
            if(width == 2)
            {
                // a 16-bit read with a single byte in the FIFO is undefined on the chip; here the high byte is 0
                value |= (uint)Dequeue() << 8;
            }
            Update();
            return value;
        }

        private byte Dequeue()
        {
            return receiveFifo.Count > 0 ? receiveFifo.Dequeue() : (byte)0;
        }

        private void Update()
        {
            IRQ.Set(transmitEmptyInterrupt.Value && TransmitEmpty
                || receiveNotEmptyInterrupt.Value && ReceiveNotEmpty
                || errorInterrupt.Value && overrun);
        }

        private void Warn(bool condition, string what)
        {
            if(condition)
            {
                this.Log(LogLevel.Warning, "{0} is not modelled", what);
            }
        }

        private void DefineRegisters()
        {
            Registers.Control1.Define(registers)
                .WithFlag(0, name: "CPHA")
                .WithFlag(1, name: "CPOL")
                .WithFlag(2, out master, name: "MSTR")
                .WithValueField(3, 3, name: "BR")
                .WithFlag(6, out enable, changeCallback: (_, value) =>
                {
                    if(!value)
                    {
                        return;
                    }
                    Warn(!master.Value, "Slave mode");
                    SendQueued();
                }, name: "SPE")
                .WithFlag(7, writeCallback: (_, value) => Warn(value, "LSB first"), name: "LSBFIRST")
                .WithFlag(8, name: "SSI")
                .WithFlag(9, name: "SSM")
                .WithFlag(10, writeCallback: (_, value) => Warn(value, "Receive only"), name: "RXONLY")
                .WithFlag(11, name: "CRCL")
                .WithFlag(12, name: "CRCNEXT")
                .WithFlag(13, writeCallback: (_, value) => Warn(value, "CRC"), name: "CRCEN")
                .WithFlag(14, out bidirectionalOutput, name: "BIDIOE")
                .WithFlag(15, out bidirectionalMode, name: "BIDIMODE")
                .WithReservedBits(16, 16)
                .WithWriteCallback((_, __) =>
                {
                    Warn(bidirectionalMode.Value && !bidirectionalOutput.Value, "One-line receive");
                    Update();
                });

            Registers.Control2.Define(registers, 0x0700)
                .WithFlag(0, out receiveDma, name: "RXDMAEN")
                .WithFlag(1, name: "TXDMAEN")
                .WithFlag(2, name: "SSOE")
                .WithFlag(3, name: "NSSP")
                .WithFlag(4, writeCallback: (_, value) => Warn(value, "TI mode"), name: "FRF")
                .WithFlag(5, out errorInterrupt, name: "ERRIE")
                .WithFlag(6, out receiveNotEmptyInterrupt, name: "RXNEIE")
                .WithFlag(7, out transmitEmptyInterrupt, name: "TXEIE")
                .WithValueField(8, 4, out dataSize, writeCallback: (_, value) =>
                {
                    if(value < 3)
                    {
                        dataSize.Value = EightBits;
                    }
                    else if(value != EightBits && value != SixteenBits)
                    {
                        this.Log(LogLevel.Warning, "Frames of {0} bits are not modelled, only 8 and 16", value + 1);
                    }
                }, name: "DS")
                .WithFlag(12, out receiveThreshold, name: "FRXTH")
                .WithFlag(13, name: "LDMA_RX")
                .WithFlag(14, name: "LDMA_TX")
                .WithReservedBits(15, 17)
                .WithWriteCallback((_, __) =>
                {
                    Update();
                    RequestReceiveDma();
                });

            Registers.Status.Define(registers, 0x0002)
                .WithFlag(0, FieldMode.Read, valueProviderCallback: _ => ReceiveNotEmpty, name: "RXNE")
                .WithFlag(1, FieldMode.Read, valueProviderCallback: _ => TransmitEmpty, name: "TXE")
                .WithReservedBits(2, 2)
                .WithFlag(4, FieldMode.Read, valueProviderCallback: _ => false, name: "CRCERR")
                .WithFlag(5, FieldMode.Read, valueProviderCallback: _ => false, name: "MODF")
                .WithFlag(6, FieldMode.Read, valueProviderCallback: _ => overrun, name: "OVR")
                .WithFlag(7, FieldMode.Read, valueProviderCallback: _ => false, name: "BSY")
                .WithFlag(8, FieldMode.Read, valueProviderCallback: _ => false, name: "FRE")
                .WithValueField(9, 2, FieldMode.Read, valueProviderCallback: _ => (ulong)Level(receiveFifo.Count), name: "FRLVL")
                .WithValueField(11, 2, FieldMode.Read, valueProviderCallback: _ => (ulong)Level(transmitFifo.Count), name: "FTLVL")
                .WithReservedBits(13, 19)
                .WithReadCallback((_, __) =>
                {
                    if(overrun && dataReadInOverrun)
                    {
                        overrun = false;
                        dataReadInOverrun = false;
                        Update();
                    }
                });

            Registers.CRCPolynomial.Define(registers, 0x0007)
                .WithValueField(0, 16, name: "CRCPOLY")
                .WithReservedBits(16, 16);

            Registers.ReceivedCRC.Define(registers)
                .WithValueField(0, 16, FieldMode.Read, valueProviderCallback: _ => 0, name: "RXCRC")
                .WithReservedBits(16, 16);

            Registers.TransmittedCRC.Define(registers)
                .WithValueField(0, 16, FieldMode.Read, valueProviderCallback: _ => 0, name: "TXCRC")
                .WithReservedBits(16, 16);

            Registers.I2SConfiguration.Define(registers)
                .WithValueField(0, 11, name: "I2SCFGR")
                .WithFlag(11, writeCallback: (_, value) => Warn(value, "I2S"), name: "I2SMOD")
                .WithReservedBits(12, 20);

            Registers.I2SPrescaler.Define(registers, 0x0002)
                .WithValueField(0, 10, name: "I2SPR")
                .WithReservedBits(10, 22);
        }

        private bool overrun;
        private bool dataReadInOverrun;
        private bool warnedUnconnected;

        private IFlagRegisterField master;
        private IFlagRegisterField enable;
        private IFlagRegisterField bidirectionalOutput;
        private IFlagRegisterField bidirectionalMode;
        private IFlagRegisterField receiveDma;
        private IFlagRegisterField errorInterrupt;
        private IFlagRegisterField receiveNotEmptyInterrupt;
        private IFlagRegisterField transmitEmptyInterrupt;
        private IFlagRegisterField receiveThreshold;
        private IValueRegisterField dataSize;

        private readonly DoubleWordRegisterCollection registers;
        private readonly Queue<byte> receiveFifo = new Queue<byte>();
        private readonly FrameQueue transmitFifo = new FrameQueue();

        private const long DataRegister = 0x0C;
        private const int FifoBytes = 4;
        private const ulong EightBits = 7;
        private const ulong SixteenBits = 15;

        private class FrameQueue
        {
            public void Enqueue(byte[] frame)
            {
                frames.Enqueue(frame);
                Count += frame.Length;
            }

            public byte[] Dequeue()
            {
                var frame = frames.Dequeue();
                Count -= frame.Length;
                return frame;
            }

            public void Clear()
            {
                frames.Clear();
                Count = 0;
            }

            public int Count { get; private set; }

            private readonly Queue<byte[]> frames = new Queue<byte[]>();
        }

        private enum Registers
        {
            Control1 = 0x00,
            Control2 = 0x04,
            Status = 0x08,
            Data = 0x0C,
            CRCPolynomial = 0x10,
            ReceivedCRC = 0x14,
            TransmittedCRC = 0x18,
            I2SConfiguration = 0x1C,
            I2SPrescaler = 0x20,
        }
    }
}

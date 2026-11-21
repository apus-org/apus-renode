using System.Collections.Generic;
using System.Linq;

namespace Antmicro.Renode.Peripherals.SPI
{
    public class SpiRecorder : ISPIPeripheral
    {
        public byte Transmit(byte data)
        {
            received.Add(data);
            return (byte)(data + 1);
        }

        public void FinishTransmission()
        {
        }

        public void Reset()
        {
            received.Clear();
        }

        public string Received => string.Join(" ", received.Select(b => b.ToString("X2")));

        private readonly List<byte> received = new List<byte>();
    }
}

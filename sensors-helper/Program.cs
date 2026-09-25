using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Security.Principal;
using System.Text;
using System.Threading;
using LibreHardwareMonitor.Hardware;

namespace CoreviewSensors
{
    internal sealed class UpdateVisitor : IVisitor
    {
        public void VisitComputer(IComputer computer) => computer.Traverse(this);

        public void VisitHardware(IHardware hardware)
        {
            hardware.Update();
            foreach (IHardware sub in hardware.SubHardware)
            {
                sub.Accept(this);
            }
        }

        public void VisitSensor(ISensor sensor) { }

        public void VisitParameter(IParameter parameter) { }
    }

    internal static class Program
    {
        private static string Quote(string value)
        {
            var sb = new StringBuilder("\"");
            foreach (char c in value ?? string.Empty)
            {
                switch (c)
                {
                    case '"': sb.Append("\\\""); break;
                    case '\\': sb.Append("\\\\"); break;
                    case '\n': sb.Append("\\n"); break;
                    case '\r': sb.Append("\\r"); break;
                    case '\t': sb.Append("\\t"); break;
                    default:
                        if (c < 0x20) sb.Append("\\u").Append(((int)c).ToString("x4"));
                        else sb.Append(c);
                        break;
                }
            }
            return sb.Append('"').ToString();
        }

        private static string Number(float? value)
        {
            if (!value.HasValue || float.IsNaN(value.Value) || float.IsInfinity(value.Value)) return "null";
            return value.Value.ToString("0.###", CultureInfo.InvariantCulture);
        }

        private static bool Wanted(SensorType type)
        {
            return true;
        }

        private static void Collect(IHardware hardware, StringBuilder sb, ref bool first)
        {
            foreach (ISensor sensor in hardware.Sensors)
            {
                if (!Wanted(sensor.SensorType) || !sensor.Value.HasValue) continue;
                if (!first) sb.Append(',');
                first = false;
                sb.Append("{\"hw\":").Append(Quote(hardware.Name))
                  .Append(",\"ht\":").Append(Quote(hardware.HardwareType.ToString()))
                  .Append(",\"st\":").Append(Quote(sensor.SensorType.ToString()))
                  .Append(",\"name\":").Append(Quote(sensor.Name))
                  .Append(",\"value\":").Append(Number(sensor.Value))
                  .Append(",\"max\":").Append(Number(sensor.Max))
                  .Append('}');
            }
            foreach (IHardware sub in hardware.SubHardware)
            {
                Collect(sub, sb, ref first);
            }
        }

        private static string Snapshot(Computer computer, bool admin)
        {
            var sb = new StringBuilder();
            sb.Append("{\"ok\":true,\"admin\":").Append(admin ? "true" : "false").Append(",\"sensors\":[");
            bool first = true;
            foreach (IHardware hardware in computer.Hardware)
            {
                Collect(hardware, sb, ref first);
            }
            return sb.Append("]}").ToString();
        }

        private static void WatchParent()
        {
            var thread = new Thread(() =>
            {
                try
                {
                    while (Console.In.Read() >= 0) { }
                }
                catch (IOException) { }
                Environment.Exit(0);
            })
            {
                IsBackground = true
            };
            thread.Start();
        }

        private static int Main()
        {
            Console.OutputEncoding = new UTF8Encoding(false);
            bool admin;
            using (WindowsIdentity identity = WindowsIdentity.GetCurrent())
            {
                admin = new WindowsPrincipal(identity).IsInRole(WindowsBuiltInRole.Administrator);
            }

            var computer = new Computer
            {
                IsCpuEnabled = true,
                IsGpuEnabled = true,
                IsMemoryEnabled = true,
                IsMotherboardEnabled = true,
                IsControllerEnabled = true,
                IsStorageEnabled = true,
                IsBatteryEnabled = true,
                IsNetworkEnabled = false,
                IsPsuEnabled = false
            };

            try
            {
                computer.Open();
            }
            catch (Exception ex)
            {
                Console.WriteLine("{\"ok\":false,\"admin\":" + (admin ? "true" : "false") + ",\"error\":" + Quote(ex.Message) + "}");
                return 1;
            }

            WatchParent();
            var visitor = new UpdateVisitor();
            while (true)
            {
                try
                {
                    computer.Accept(visitor);
                    Console.WriteLine(Snapshot(computer, admin));
                    Console.Out.Flush();
                }
                catch (Exception ex)
                {
                    Console.WriteLine("{\"ok\":false,\"admin\":" + (admin ? "true" : "false") + ",\"error\":" + Quote(ex.Message) + "}");
                    Console.Out.Flush();
                }
                Thread.Sleep(1500);
            }
        }
    }
}

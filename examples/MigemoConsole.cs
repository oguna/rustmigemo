using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Runtime.InteropServices;
using System.IO;

namespace MigemoConsoleApp
{
    sealed class Migemo
    {
        [StructLayout(LayoutKind.Sequential, Pack = 8)]
        struct MigemoDescription
        {
            public IntPtr Dictionary;
            public IntPtr ResultPtr;
            public uint ResultSize;
        }

        [DllImport("rustmigemo.dll", EntryPoint = "load")]
        static unsafe extern MigemoDescription Load(byte* buffer, uint len);

        [DllImport("rustmigemo.dll", EntryPoint = "destroy")]
        static unsafe extern void Destroy(ref MigemoDescription migemo);

        [DllImport("rustmigemo.dll", EntryPoint = "query")]
        static unsafe extern bool Query(ref MigemoDescription migemo, byte* buffer, uint len);
      
        private MigemoDescription migemoDescription;

        public unsafe static Migemo Load(byte[] bytes)
        {
            fixed (byte* pByte = bytes)
            {
                return new Migemo
                {
                    migemoDescription = Load(pByte, (uint)bytes.Length)
                };
            }
        }

        public unsafe string Query(string word)
        {
            var textBytes = Encoding.UTF8.GetBytes(word);
            fixed (byte* pText = textBytes)
            {
                Query(ref migemoDescription, pText, (uint)textBytes.Length);
            }
            byte[] buffer = new byte[migemoDescription.ResultSize];
            Marshal.Copy(migemoDescription.ResultPtr, buffer, 0, buffer.Length);
            return Encoding.UTF8.GetString(buffer);
        }

        ~Migemo()
        {
            Destroy(ref migemoDescription);
        }
    }

    class Program
    {
        static void Main(string[] args)
        {
            var path = "migemo-compact-dict";
            var bytes = File.ReadAllBytes(path);
            var migemo = Migemo.Load(bytes);
            string line;
            Console.Write("QUERY: ");
            while ((line = Console.ReadLine()) != null && line.Length > 0)
            {
                Console.WriteLine("PETTERN: {0}", migemo.Query(line));
                Console.Write("QUERY: ");
            }
        }
    }
}
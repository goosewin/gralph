using System;
using System.Diagnostics;

namespace Gralph.State;

public interface IProcessInspector
{
    bool IsAlive(int pid);
}

public sealed class ProcessInspector : IProcessInspector
{
    public bool IsAlive(int pid)
    {
        if (pid <= 0)
        {
            return false;
        }

        try
        {
            using var process = Process.GetProcessById(pid);
            return !process.HasExited;
        }
        catch (ArgumentException)
        {
            return false;
        }
        catch (InvalidOperationException)
        {
            return false;
        }
    }
}

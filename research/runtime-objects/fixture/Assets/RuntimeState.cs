using System;
using System.IO;
using UnityEngine;
using UnityEngine.Scripting;

// Owned runtime correctness fixture; these names/layouts are never adapter inputs.
[Preserve] public class BaseState { public float Reserve = 4.25f; }
[Preserve] public sealed class ActorState : BaseState
{
    public int Identity;
    public int Counter;
    public float HiddenScale;
    public ActorState Peer;
    public ActorState(int id) { Identity = id; Counter = id * 17; HiddenScale = id + 0.375f; }
}
[Preserve] public sealed class RuntimeState : MonoBehaviour
{
    public static ActorState Active;
    public static ActorState Companion;
    public static int Generation;
    private string folder;
    private float nextReplace;
    private float nextRecord;
    void Awake()
    {
        folder = Environment.GetEnvironmentVariable("REVERSEBOLT_FIXTURE_OUTPUT");
        if (String.IsNullOrEmpty(folder)) throw new InvalidOperationException("Output folder required");
        Directory.CreateDirectory(folder);
        Replace();
    }
    void Replace()
    {
        Generation++;
        Active = new ActorState(Generation * 2);
        Companion = new ActorState(Generation * 2 + 1);
        Active.Peer = Companion;
        Companion.Peer = Active;
        nextReplace = Time.realtimeSinceStartup + 1f;
    }
    void Update()
    {
        if (File.Exists(Path.Combine(folder, "stop"))) { Application.Quit(); return; }
        if (Time.realtimeSinceStartup >= nextReplace) { Replace(); GC.Collect(); }
        Active.Counter++;
        if (Time.realtimeSinceStartup >= nextRecord)
        {
            nextRecord = Time.realtimeSinceStartup + 0.1f;
            File.AppendAllText(Path.Combine(folder, "oracle.jsonl"),
                "{\"generation\":" + Generation + ",\"active\":" + Active.Identity +
                ",\"companion\":" + Companion.Identity + ",\"counter\":" + Active.Counter + "}\n");
        }
    }
}

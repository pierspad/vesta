//  Headless stubs for benchmarking subs2srs without its WinForms GUI.
//
//  These replace exactly three GUI-coupled types so the *real* subs2srs
//  generation code (WorkerSubs / WorkerSrs / WorkerAudio / WorkerSnapshot /
//  WorkerVideo / Utils*) compiles and runs from a console with no X display:
//
//    * DialogProgress  -> a no-op progress sink. The original's static helpers
//                         are already guarded by `IsHandleCreated`, so a never-
//                         shown dialog is a no-op anyway; this avoids needing a
//                         display at all. `Cancel` is always false.
//    * UtilsMsg        -> prints to the console instead of popping MessageBoxes.
//    * TagLib (shim)   -> lets UtilsAudio.tagAudio compile; ID3 tagging becomes
//                         a no-op. Tagging is not part of the extraction work
//                         being benchmarked (and the original wraps it in a
//                         try/catch that "ignores and moves on").
//
//  NOTHING in the subs2srs generation/extraction logic is modified: same
//  parsing, same matching, same sequential ffmpeg calls. Honest comparison.

using System;
using System.Diagnostics;

namespace subs2srs
{
    /// <summary>Headless no-op replacement for the WinForms progress dialog.</summary>
    public class DialogProgress
    {
        public bool Cancel { get { return false; } set { } }
        public int StepsTotal { get; set; }
        public bool IsHandleCreated { get { return false; } }

        public static void updateProgressInvoke(DialogProgress d, int progress, string text) { }
        public static void updateProgressInvoke(DialogProgress d, string text) { }
        public static void updateDetailedProgressInvoke(DialogProgress d, int progress, InfoFFmpegProgress p) { }
        public static void nextStepInvoke(DialogProgress d, int step, string stepName) { }
        public static void enableDetailInvoke(DialogProgress d, bool enabled) { }
        public static bool getCancelInvoke(DialogProgress d) { return false; }
        public static void setDuration(DialogProgress d, DateTime duration) { }

        /// <summary>A no-op stderr handler (the original streamed ffmpeg output to the UI).</summary>
        public static DataReceivedEventHandler getFFmpegOutputHandler(DialogProgress d)
        {
            return (sender, e) => { };
        }
    }

    /// <summary>Console replacement for the WinForms message-box helper.</summary>
    public static class UtilsMsg
    {
        public static void showErrMsg(string msg) { Console.Error.WriteLine("[subs2srs] ERROR: " + msg); }
        public static void showInfoMsg(string msg) { Console.Error.WriteLine("[subs2srs] " + msg); }
        public static bool showConfirm(string msg) { return true; }
    }
}

namespace TagLib
{
    /// <summary>Minimal compile/no-op shim for taglib-sharp (ID3 tagging only).</summary>
    public class File
    {
        public Tag Tag = new Tag();
        public static File Create(string path) { return new File(); }
        public void Save() { }
    }

    public class Tag
    {
        public string[] Performers { get; set; }
        public string Album { get; set; }
        public string Title { get; set; }
        public string[] Genres { get; set; }
        public uint Track { get; set; }
        public uint TrackCount { get; set; }
        public string Lyrics { get; set; }
    }
}

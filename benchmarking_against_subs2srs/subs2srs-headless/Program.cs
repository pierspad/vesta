//  Headless console driver for subs2srs — the "custom injector".
//
//  It configures Settings.Instance exactly as the GUI would, then runs the SAME
//  sequence the GUI's SubsProcessor.bw_DoWork runs:
//
//      combineAllSubs -> inactivateLines -> genSrs
//                     -> [AudioClips.Enabled]  genAudioClip
//                     -> [Snapshots.Enabled]   genSnapshots
//                     -> [VideoClips.Enabled]  genVideoClip
//
//  No worker logic is touched; subs2srs stays single-threaded and sequential,
//  using its own ffmpeg invocations. Vesta is the one allowed to parallelise.

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;

namespace subs2srs
{
    static class HeadlessProgram
    {
        static int Main(string[] args)
        {
            string target = null, native = null, video = null, output = null;
            string deck = "BenchmarkDeck";
            bool genAudio = true, genSnap = true, genVideo = true;

            for (int i = 0; i < args.Length; i++)
            {
                switch (args[i])
                {
                    case "--target": target = args[++i]; break;
                    case "--native": native = args[++i]; break;
                    case "--video": video = args[++i]; break;
                    case "--output": output = args[++i]; break;
                    case "--deck": deck = args[++i]; break;
                    case "--no-audio": genAudio = false; break;
                    case "--no-snapshots": genSnap = false; break;
                    case "--no-video": genVideo = false; break;
                    default:
                        Console.Error.WriteLine("Unknown arg: " + args[i]);
                        break;
                }
            }

            if (target == null || output == null)
            {
                Console.Error.WriteLine(
                    "Usage: subs2srs-headless --target <t.srt> [--native <n.srt>] --video <v> --output <dir> [--deck <name>] [--no-audio|--no-snapshots|--no-video]");
                return 2;
            }

            // ── Configure Settings exactly as the GUI would ──────────────────
            Settings s = Settings.Instance;
            s.OutputDir = output;
            s.DeckName = deck;
            s.EpisodeStartNumber = 1;

            // Target / native subtitle streams (one episode).
            s.Subs[0].Files = new string[] { target };
            s.Subs[0].Encoding = "utf-8";
            if (native != null)
            {
                s.Subs[1].Files = new string[] { native };
                s.Subs[1].Encoding = "utf-8";
            }
            else
            {
                s.Subs[1].Files = new string[0];
            }

            // Media: video file drives snapshots, video clips and (ripped) audio.
            s.VideoClips.Files = new string[] { video };
            s.VideoClips.Enabled = genVideo;
            // The GUI sets the audio stream after probing the video; headless we
            // pick the first audio stream (ffmpeg 0:a:0), matching a default run.
            s.VideoClips.AudioStream = new InfoStream("0", "0", "", "");

            s.Snapshots.Enabled = genSnap;

            s.AudioClips.Enabled = genAudio;
            s.AudioClips.UseAudioFromVideo = true;
            s.AudioClips.Files = new string[0];

            // ── Replicate SubsProcessor.bw_DoWork (no GUI, no BackgroundWorker) ─
            string mediaDir = string.Format("{0}{1}{2}.media",
                output, Path.DirectorySeparatorChar, deck);
            Directory.CreateDirectory(mediaDir);

            WorkerVars wv = new WorkerVars(null, mediaDir, WorkerVars.SubsProcessingType.Normal);
            DialogProgress dp = new DialogProgress();
            WorkerSubs subs = new WorkerSubs();

            Stopwatch sw = Stopwatch.StartNew();

            // 1) Parse + combine subtitles
            List<List<InfoCombined>> combined = subs.combineAllSubs(wv, dp);
            if (combined == null) { Console.Error.WriteLine("SUBS2SRS_BENCHMARK_ERROR: combineAllSubs returned null"); return 1; }
            wv.CombinedAll = combined;

            int totalLines = 0;
            foreach (var ep in wv.CombinedAll) totalLines += ep.Count;
            if (totalLines == 0) { Console.Error.WriteLine("SUBS2SRS_BENCHMARK_ERROR: no lines parsed"); return 1; }

            // 2) Inactivate filtered lines
            combined = subs.inactivateLines(wv, dp);
            if (combined == null) { Console.Error.WriteLine("SUBS2SRS_BENCHMARK_ERROR: inactivateLines returned null"); return 1; }
            wv.CombinedAll = combined;

            // 3) Generate the SRS (TSV) import file
            if (!new WorkerSrs().genSrs(wv, dp)) { Console.Error.WriteLine("SUBS2SRS_BENCHMARK_ERROR: genSrs failed"); return 1; }

            // Keep a context-bearing copy; media workers operate on context-stripped lines
            // (ContextLeadingCount/TrailingCount default to 0, matching the Vesta benchmark).
            List<List<InfoCombined>> withContext = ObjectCopier.Clone<List<List<InfoCombined>>>(wv.CombinedAll);

            // 4) Audio clips
            if (s.AudioClips.Enabled)
            {
                wv.CombinedAll = subs.removeContextOnlyLines(withContext);
                if (!new WorkerAudio().genAudioClip(wv, dp)) { Console.Error.WriteLine("SUBS2SRS_BENCHMARK_ERROR: genAudioClip failed"); return 1; }
            }

            // 5) Snapshots
            if (s.Snapshots.Enabled)
            {
                wv.CombinedAll = subs.removeContextOnlyLines(withContext);
                if (!new WorkerSnapshot().genSnapshots(wv, dp)) { Console.Error.WriteLine("SUBS2SRS_BENCHMARK_ERROR: genSnapshots failed"); return 1; }
            }

            // 6) Video clips
            if (s.VideoClips.Enabled)
            {
                wv.CombinedAll = subs.removeContextOnlyLines(withContext);
                if (!new WorkerVideo().genVideoClip(wv, dp)) { Console.Error.WriteLine("SUBS2SRS_BENCHMARK_ERROR: genVideoClip failed"); return 1; }
            }

            sw.Stop();

            // Count produced media for sanity reporting.
            int mp3 = Directory.GetFiles(mediaDir, "*.mp3").Length;
            int jpg = Directory.GetFiles(mediaDir, "*.jpg").Length;
            int mp4 = Directory.GetFiles(mediaDir, "*.mp4").Length
                    + Directory.GetFiles(mediaDir, "*.avi").Length;

            Console.WriteLine("SUBS2SRS_BENCHMARK_SUCCESS: " + sw.ElapsedMilliseconds + " ms");
            Console.WriteLine("SUBS2SRS_BENCHMARK_CARDS: lines=" + totalLines +
                " audio=" + mp3 + " snapshots=" + jpg + " video=" + mp4);
            return 0;
        }
    }
}

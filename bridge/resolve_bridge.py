#!/usr/bin/env python3
"""
Aeon-Cut DaVinci Resolve Bridge
High-performance interface connecting to DaVinci Resolve 21 / 20 / 19 Scripting API.
Provides:
  - Connection & timeline status detection
  - Active clip inspection (media file path, timecode, framerate)
  - Native subclip assembly via CreateEmptyTimeline & AppendToTimeline
  - Dual-tier marker placement (Clip & Timeline markers)
"""

import sys
import os
import json
import time
import argparse
from typing import Optional, Dict, Any, List

def init_resolve_api():
    """Dynamically set up environment variables and import DaVinciResolveScript."""
    possible_paths = [
        "/opt/resolve/Developer/Scripting/Modules",
        "/home/resolve/Developer/Scripting/Modules",
        os.path.expanduser("~/.local/share/DaVinciResolve/Developer/Scripting/Modules"),
        "/Library/Application Support/Blackmagic Design/DaVinci Resolve/Developer/Scripting/Modules",
        r"C:\ProgramData\Blackmagic Design\DaVinci Resolve\Support\Developer\Scripting\Modules"
    ]
    for p in possible_paths:
        if os.path.exists(p) and p not in sys.path:
            sys.path.append(p)

    if sys.platform.startswith("linux"):
        os.environ.setdefault("RESOLVE_SCRIPT_API", "/opt/resolve/Developer/Scripting")
        os.environ.setdefault("RESOLVE_SCRIPT_LIB", "/opt/resolve/libs/Fusion/fusionscript.so")
    elif sys.platform == "darwin":
        os.environ.setdefault("RESOLVE_SCRIPT_API", "/Library/Application Support/Blackmagic Design/DaVinci Resolve/Developer/Scripting")
        os.environ.setdefault("RESOLVE_SCRIPT_LIB", "/Applications/DaVinci Resolve/DaVinci Resolve.app/Contents/Libraries/Fusion/fusionscript.so")
    elif sys.platform == "win32":
        os.environ.setdefault("RESOLVE_SCRIPT_API", r"C:\ProgramData\Blackmagic Design\DaVinci Resolve\Support\Developer\Scripting")
        os.environ.setdefault("RESOLVE_SCRIPT_LIB", r"C:\Program Files\Blackmagic Design\DaVinci Resolve\fusionscript.dll")

    try:
        import DaVinciResolveScript as dvr_script
        return dvr_script
    except ImportError:
        return None

def get_resolve_instance():
    dvr = init_resolve_api()
    if not dvr:
        return None
    try:
        return dvr.scriptapp("Resolve")
    except Exception:
        return None

def timecode_to_frames(tc_str: str, fps: float) -> int:
    """Converts HH:MM:SS:FF timecode string to frame number."""
    if not tc_str:
        return 0
    try:
        parts = [int(p) for p in tc_str.strip().split(":")]
        if len(parts) == 4:
            h, m, s, f = parts
            return int(round((h * 3600 + m * 60 + s) * fps + f))
    except Exception:
        pass
    return 0

def get_status() -> Dict[str, Any]:
    resolve = get_resolve_instance()
    if not resolve:
        return {
            "connected": False,
            "error": "DaVinci Resolve is not running or scripting API is disabled."
        }
    
    pm = resolve.GetProjectManager()
    if not pm:
        return {"connected": True, "project": None, "timeline": None}
    
    project = pm.GetCurrentProject()
    if not project:
        return {"connected": True, "project": None, "timeline": None}
    
    timeline = project.GetCurrentTimeline()
    
    timeline_name = timeline.GetName() if timeline else None
    timeline_fps = float(timeline.GetSetting("timelineFrameRate") or 24.0) if timeline else 24.0
    start_frame = timeline.GetStartFrame() if timeline else 0
    end_frame = timeline.GetEndFrame() if timeline else 0

    return {
        "connected": True,
        "version": resolve.GetVersionString(),
        "project_name": project.GetName(),
        "timeline_name": timeline_name,
        "timeline_fps": timeline_fps,
        "timeline_start_frame": start_frame,
        "timeline_end_frame": end_frame,
    }

def get_selected_clip() -> Dict[str, Any]:
    status = get_status()
    if not status.get("connected"):
        return status
    
    resolve = get_resolve_instance()
    project = resolve.GetProjectManager().GetCurrentProject()
    if not project:
        return {"error": "No open project found."}
    
    timeline = project.GetCurrentTimeline()
    if not timeline:
        return {"error": "No active timeline found."}
    
    selected_clips = timeline.GetSelectedClips()
    if not selected_clips:
        return {
            "error": "No clip selected in timeline. Please select a video/audio clip.",
            "has_selection": False
        }
    
    clip = selected_clips[0]
    mp_item = clip.GetMediaPoolItem()
    
    file_path = ""
    fps = float(timeline.GetSetting("timelineFrameRate") or 24.0)
    start_tc_frames = 0
    
    if mp_item:
        file_path = mp_item.GetClipProperty("File Path") or ""
        mp_fps = mp_item.GetClipProperty("FPS")
        if mp_fps:
            try:
                fps = float(mp_fps)
            except ValueError:
                pass
        start_tc = mp_item.GetClipProperty("Start TC") or ""
        start_tc_frames = timecode_to_frames(start_tc, fps)
    
    timeline_start = clip.GetStart()
    timeline_end = clip.GetEnd()
    duration = clip.GetDuration()
    
    source_start = clip.GetSourceStartFrame()
    source_end = clip.GetSourceEndFrame()
    
    base_source_frame = source_start if (source_start and source_start > 0) else start_tc_frames

    return {
        "success": True,
        "has_selection": True,
        "clip_name": clip.GetName(),
        "file_path": file_path,
        "fps": fps,
        "timeline_start_frame": timeline_start,
        "timeline_end_frame": timeline_end,
        "duration_frames": duration,
        "source_start_frame": base_source_frame,
        "source_end_frame": source_end,
        "timeline_global_start": timeline.GetStartFrame(),
        "timeline_name": timeline.GetName()
    }

def apply_cuts_clean_timeline(speech_segments: List[Dict[str, Any]], timeline_name_suffix: str = "AeonCut") -> Dict[str, Any]:
    """
    Creates a new cut timeline from speech segments using official
    CreateEmptyTimeline + AppendToTimeline([subclips]).
    Switches DaVinci Resolve's active viewer directly to the new cut timeline.
    """
    resolve = get_resolve_instance()
    if not resolve:
        return {"error": "DaVinci Resolve not connected."}
    
    project = resolve.GetProjectManager().GetCurrentProject()
    timeline = project.GetCurrentTimeline()
    if not timeline:
        return {"error": "No active timeline found."}
    
    selected_clips = timeline.GetSelectedClips()
    if not selected_clips:
        return {"error": "No clip selected. Please select a clip in the timeline."}
    
    clip = selected_clips[0]
    mp_item = clip.GetMediaPoolItem()
    if not mp_item:
        return {"error": "Clip MediaPoolItem reference not found."}
    
    media_pool = project.GetMediaPool()
    orig_name = timeline.GetName()
    new_timeline_name = f"{orig_name} - {timeline_name_suffix}"
    
    # 1. Create an empty timeline
    new_timeline = media_pool.CreateEmptyTimeline(new_timeline_name)
    if not new_timeline:
        unique_suffix = f"{timeline_name_suffix}_{int(time.time()) % 10000}"
        new_timeline_name = f"{orig_name} - {unique_suffix}"
        new_timeline = media_pool.CreateEmptyTimeline(new_timeline_name)
        
    if not new_timeline:
        return {"error": f"Failed to create new timeline '{new_timeline_name}'."}
    
    # 2. Switch Resolve viewer to the new timeline
    project.SetCurrentTimeline(new_timeline)
    
    # 3. Assemble subclips
    subclips = []
    total_speech_frames = 0
    for seg in speech_segments:
        s_start = int(round(seg.get("source_start_frame", 0)))
        s_end = int(round(seg.get("source_end_frame", 0)))
        dur = s_end - s_start
        if dur <= 0:
            continue
        
        subclips.append({
            "mediaPoolItem": mp_item,
            "startFrame": s_start,
            "endFrame": s_end,
        })
        total_speech_frames += dur

    if not subclips:
        return {"error": "No valid speech segments found."}

    # 4. Append subclips to timeline
    success = media_pool.AppendToTimeline(subclips)
    
    if success:
        return {
            "success": True,
            "new_timeline_name": new_timeline_name,
            "segments_count": len(subclips),
            "total_frames": total_speech_frames
        }
    else:
        added = 0
        for sc in subclips:
            if media_pool.AppendToTimeline([sc]):
                added += 1
        if added > 0:
            return {
                "success": True,
                "new_timeline_name": new_timeline_name,
                "segments_count": added,
                "total_frames": total_speech_frames
            }
        return {"error": "Failed to append cut subclips to timeline."}

def apply_cuts_active_timeline(speech_segments: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Appends cut speech subclips directly to the current active timeline."""
    resolve = get_resolve_instance()
    if not resolve:
        return {"error": "DaVinci Resolve not connected."}
    
    project = resolve.GetProjectManager().GetCurrentProject()
    timeline = project.GetCurrentTimeline()
    if not timeline:
        return {"error": "No active timeline found."}
    
    selected_clips = timeline.GetSelectedClips()
    if not selected_clips:
        return {"error": "No clip selected."}
    
    clip = selected_clips[0]
    mp_item = clip.GetMediaPoolItem()
    if not mp_item:
        return {"error": "MediaPoolItem reference not found."}
    
    media_pool = project.GetMediaPool()
    
    subclips = []
    total_frames = 0
    for seg in speech_segments:
        s_start = int(round(seg.get("source_start_frame", 0)))
        s_end = int(round(seg.get("source_end_frame", 0)))
        dur = s_end - s_start
        if dur <= 0:
            continue
        subclips.append({
            "mediaPoolItem": mp_item,
            "startFrame": s_start,
            "endFrame": s_end
        })
        total_frames += dur
        
    res = media_pool.AppendToTimeline(subclips)
    if not res:
        added = 0
        chunk_size = 100
        for i in range(0, len(subclips), chunk_size):
            chunk = subclips[i:i + chunk_size]
            if media_pool.AppendToTimeline(chunk):
                added += len(chunk)
            else:
                for sc in chunk:
                    if media_pool.AppendToTimeline([sc]):
                        added += 1
        res = added > 0
        count = added
    else:
        count = len(subclips)

    if not res:
        return {"error": "Failed to append cut subclips to active timeline."}

    return {
        "success": True,
        "segments_count": count,
        "total_frames": total_frames
    }

def apply_markers(speech_segments: List[Dict[str, Any]], marker_color: str = "Cyan") -> Dict[str, Any]:
    """Adds markers both to the clip itself (relative offset) and the timeline (absolute frame)."""
    resolve = get_resolve_instance()
    if not resolve:
        return {"error": "DaVinci Resolve not connected."}
    
    project = resolve.GetProjectManager().GetCurrentProject()
    timeline = project.GetCurrentTimeline()
    if not timeline:
        return {"error": "No active timeline found."}
    
    selected_clips = timeline.GetSelectedClips()
    target_clip = selected_clips[0] if selected_clips else None
    
    timeline_global_start = timeline.GetStartFrame()
    clip_start_frame = target_clip.GetStart() if target_clip else 0
    
    clip_markers_count = 0
    timeline_markers_count = 0
    
    for idx, seg in enumerate(speech_segments):
        start_sec = seg.get("start_seconds", 0.0)
        end_sec = seg.get("end_seconds", start_sec + 0.5)
        dur_sec = max(end_sec - start_sec, 0.05)
        
        fps = float(timeline.GetSetting("timelineFrameRate") or 24.0)
        dur_frames = max(int(round(dur_sec * fps)), 1)
        rel_frame = int(round(start_sec * fps))
        
        # 1. Add marker to the clip itself
        if target_clip:
            try:
                ok = target_clip.AddMarker(rel_frame, marker_color, f"Speech {idx+1}", f"{dur_sec:.2f}s", dur_frames, "aeon_cut")
                if ok:
                    clip_markers_count += 1
            except Exception:
                pass
        
        # 2. Add marker to the timeline
        abs_timeline_frame = timeline_global_start + clip_start_frame + rel_frame
        try:
            ok2 = timeline.AddMarker(abs_timeline_frame, marker_color, f"Speech {idx+1}", f"{dur_sec:.2f}s", dur_frames, "aeon_cut")
            if ok2:
                timeline_markers_count += 1
        except Exception:
            pass

    total_added = max(clip_markers_count, timeline_markers_count)
    if total_added > 0:
        return {
            "success": True,
            "markers_added": total_added,
            "clip_markers": clip_markers_count,
            "timeline_markers": timeline_markers_count
        }
    else:
        return {"error": "Failed to add markers. Timecode bounds out of range."}

def main():
    parser = argparse.ArgumentParser(description="Aeon-Cut DaVinci Resolve Bridge")
    parser.add_argument("--action", required=True, choices=["status", "get_selected_clip", "create_clean_timeline", "append_to_active", "add_markers"])
    parser.add_argument("--segments-json", help="JSON string or file path containing speech segments")
    parser.add_argument("--suffix", default="AeonCut", help="Suffix for new timeline")
    
    args = parser.parse_args()
    
    if args.action == "status":
        res = get_status()
        print(json.dumps(res, ensure_ascii=False, indent=2))
    elif args.action == "get_selected_clip":
        res = get_selected_clip()
        print(json.dumps(res, ensure_ascii=False, indent=2))
    elif args.action in ["create_clean_timeline", "append_to_active", "add_markers"]:
        if not args.segments_json:
            print(json.dumps({"error": "--segments-json is required."}))
            sys.exit(1)
        
        segments = []
        if os.path.isfile(args.segments_json):
            with open(args.segments_json, "r", encoding="utf-8") as f:
                segments = json.load(f)
        else:
            segments = json.loads(args.segments_json)
            
        if args.action == "create_clean_timeline":
            res = apply_cuts_clean_timeline(segments, args.suffix)
        elif args.action == "append_to_active":
            res = apply_cuts_active_timeline(segments)
        else:
            res = apply_markers(segments)
        print(json.dumps(res, ensure_ascii=False, indent=2))

if __name__ == "__main__":
    main()

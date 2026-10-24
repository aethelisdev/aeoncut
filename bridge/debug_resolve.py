#!/usr/bin/env python3
"""
Debug tool to inspect DaVinci Resolve 21 timeline, clips, and properties.
"""
import sys
import os
import json

paths = [
    "/opt/resolve/Developer/Scripting/Modules",
    "/Library/Application Support/Blackmagic Design/DaVinci Resolve/Developer/Scripting/Modules",
    r"C:\ProgramData\Blackmagic Design\DaVinci Resolve\Support\Developer\Scripting\Modules"
]
for p in paths:
    if os.path.exists(p) and p not in sys.path:
        sys.path.append(p)

try:
    import DaVinciResolveScript as dvr
    resolve = dvr.scriptapp("Resolve")
    if not resolve:
        print("Resolve is not running or scripting API disabled.")
        sys.exit(0)
        
    pm = resolve.GetProjectManager()
    proj = pm.GetCurrentProject()
    print("Project:", proj.GetName() if proj else "None")
    
    tl = proj.GetCurrentTimeline() if proj else None
    if not tl:
        print("No active timeline.")
        sys.exit(0)
        
    print(f"Timeline: {tl.GetName()} | StartFrame: {tl.GetStartFrame()} | EndFrame: {tl.GetEndFrame()} | StartTC: {tl.GetStartTimecode()} | FPS: {tl.GetSetting('timelineFrameRate')}")
    
    clips = tl.GetSelectedClips()
    print(f"Selected clips count: {len(clips) if clips else 0}")
    if clips:
        c = clips[0]
        mp = c.GetMediaPoolItem()
        print(f"  Clip Name: {c.GetName()}")
        print(f"  Clip Start: {c.GetStart()} | End: {c.GetEnd()} | Duration: {c.GetDuration()}")
        print(f"  Source Start Frame: {c.GetSourceStartFrame()} | Source End Frame: {c.GetSourceEndFrame()}")
        if mp:
            print(f"  MP Properties: {json.dumps(mp.GetClipProperty(), indent=2, ensure_ascii=False)}")
except Exception as e:
    print("Error:", e)

# MultiStream Director - Concept Document

## Overview

A desktop application designed to manage multiple game instances streaming to different platforms simultaneously from a single PC. Perfect for streamers who want to broadcast the same game (or different games) to Twitch, YouTube, Kick, and other platforms with complete isolation and control.

---

## Core Architecture

```
┌─────────────────────────────────────────────────┐
│         Master Control Interface (Main UI)      │
├─────────────────────────────────────────────────┤
│  [Game 1]      [Game 2]      [Game 3]          │
│  Twitch        YouTube       Kick               │
│  ▶️ Running     ▶️ Running     ⏸️ Paused          │
└─────────────────────────────────────────────────┘
         ↓              ↓              ↓
┌────────────┐  ┌────────────┐  ┌────────────┐
│ Container 1│  │ Container 2│  │ Container 3│
│ (Isolated) │  │ (Isolated) │  │ (Isolated) │
│            │  │            │  │            │
│ Game.exe   │  │ Game.exe   │  │ Game.exe   │
│ OBS.exe    │  │ OBS.exe    │  │ OBS.exe    │
│ Browser    │  │ Browser    │  │ Browser    │
│ (Twitch)   │  │ (YouTube)  │  │ (Kick)     │
│            │  │            │  │            │
│ Profile A  │  │ Profile B  │  │ Profile C  │
│ Settings1  │  │ Settings2  │  │ Settings3  │
└────────────┘  └────────────┘  └────────────┘
```

---

## Key Features

### 1. Containerized Environments

Each "slot" runs in an isolated container with:
- **Separate file system, registry, and settings** per container
- **Different accounts/credentials** per container (browser cookies isolated)
- **Resource limits** per container (CPU/RAM allocation)
- **Independent configurations** - no conflicts between instances

### 2. Master Control Panel

```
┌─────────────────────────────────────────────────┐
│ MultiStream Director                            │
├─────────────────────────────────────────────────┤
│ Slots:                                          │
│                                                 │
│ 🟢 Slot 1: falling-heads.exe                    │
│    Platform: Twitch (@username1)                │
│    OBS: 1080p60, 6000kbps                      │
│    CPU: 25% | RAM: 4GB | Upload: 8Mbps         │
│    [⏸️ Pause] [⚙️ Settings] [👁️ Preview]         │
│                                                 │
│ 🟢 Slot 2: falling-heads.exe                    │
│    Platform: YouTube (@channel2)                │
│    OBS: 1080p60, 8000kbps                      │
│    CPU: 30% | RAM: 5GB | Upload: 10Mbps        │
│    [⏸️ Pause] [⚙️ Settings] [👁️ Preview]         │
│                                                 │
│ 🔴 Slot 3: minecraft.exe                        │
│    Platform: Kick (@username3)                  │
│    OBS: 720p60, 4000kbps                       │
│    CPU: 20% | RAM: 3GB | Upload: 6Mbps         │
│    [▶️ Start] [⚙️ Settings] [👁️ Preview]         │
│                                                 │
│ [+ Add Slot] [📊 Global Stats] [💬 All Chats]   │
└─────────────────────────────────────────────────┘
```

### 3. Per-Slot Configuration

Each slot can be configured independently:

```json
{
  "slot_id": 1,
  "game": {
    "executable": "C:/Games/falling-heads.exe",
    "working_dir": "C:/Games/",
    "args": ["--fullscreen", "--fps-limit", "60"],
    "window_size": "1920x1080",
    "window_position": "0,0"
  },
  "obs": {
    "profile": "twitch_profile",
    "scene_collection": "gaming_scenes",
    "auto_start_streaming": true,
    "output": {
      "resolution": "1920x1080",
      "fps": 60,
      "bitrate": 6000
    }
  },
  "platform": {
    "name": "twitch",
    "account": "username1",
    "stream_key": "encrypted_key_1",
    "chat_enabled": true,
    "alerts_enabled": true
  },
  "resources": {
    "cpu_limit": "25%",
    "ram_limit": "4GB",
    "gpu_priority": "normal"
  },
  "audio": {
    "input_device": "Microphone (Slot 1)",
    "output_device": "Virtual Output 1",
    "game_volume": 80,
    "mic_volume": 100
  }
}
```

### 4. Unified Chat Dashboard

View and respond to all platform chats in one place:

```
┌─────────────────────────────────────────┐
│ 💬 All Chats                            │
├─────────────────────────────────────────┤
│ [Twitch] user123: Nice play!           │
│ [YouTube] viewer456: What's the combo? │
│ [Kick] fan789: LFG!!!                  │
├─────────────────────────────────────────┤
│ [Send to: ▼All] Message...    [Send]   │
└─────────────────────────────────────────┘
```

### 5. Smart Resource Management

- **Auto-detect PC specs** and recommend optimal settings
- **Recommend slot limits** based on available hardware
- **Dynamic quality adjustment** if performance drops
- **Pause non-essential slots** if one game needs more resources
- **Real-time monitoring** of CPU, RAM, GPU, network per slot

### 6. One-Click Templates

Pre-configured setups for common scenarios:

```
Templates:
├─ Triple Same Game (3 platforms)
│  └─ Same game to Twitch, YouTube, Kick with optimized settings
├─ Variety Stream (3 different games)
│  └─ Different game per platform
├─ Co-stream Setup (2 games + chatting view)
│  └─ Two game instances + browser for chat interaction
└─ Tournament (4 instances, same game, different accounts)
   └─ Multiple perspectives of same game
```

### 7. Synchronization Features

- **Start/stop all streams simultaneously**
- **Synchronized scene switching** across all OBS instances
- **Global hotkeys** affect all slots or specific slot
- **Batch updates** - change settings across multiple slots at once

---

## Implementation Options

### Option A: Electron + Windows Sandbox API

**Technology:**
- **UI:** Electron (cross-platform, web technologies)
- **Isolation:** Windows Sandbox API or lightweight VMs
- **Process management:** Node.js child processes
- **OBS control:** obs-websocket protocol

**Pros:**
- Cross-platform (Windows, macOS, Linux)
- Easy UI development with web tech
- Large ecosystem of libraries

**Cons:**
- Higher memory usage
- Slower startup time

### Option B: Tauri + Docker-like Containers (Recommended)

**Technology:**
- **UI:** Tauri (Rust backend, web frontend)
- **Isolation:** Custom container system using Windows process isolation
- **Process management:** Rust async runtime
- **OBS control:** obs-websocket via Rust client

**Pros:**
- Lightweight (much smaller than Electron)
- Better performance
- Lower memory footprint
- Built with Rust (same as your game!)

**Cons:**
- Smaller ecosystem than Electron
- More complex containerization

### Option C: Native C# WPF

**Technology:**
- **UI:** WPF (Windows-only, native controls)
- **Isolation:** Windows Containers or process namespaces
- **Process management:** .NET Process API
- **OBS control:** obs-websocket via C# client

**Pros:**
- Best Windows integration
- Native look and feel
- Excellent performance

**Cons:**
- Windows-only
- Steeper learning curve for web developers

---

## Hardware Requirements

### Minimum Specs (2 Streams @ 720p60)
- **CPU:** Ryzen 7 5800X / Intel i7-12700K (8+ cores)
- **GPU:** RTX 3060 (NVENC for hardware encoding)
- **RAM:** 16GB
- **Storage:** 512GB NVMe SSD
- **Network:** 50 Mbps upload

### Recommended Specs (3 Streams @ 1080p60)
- **CPU:** Ryzen 9 5900X / Intel i9-12900K (12+ cores)
- **GPU:** RTX 4070+ (better NVENC encoder)
- **RAM:** 32GB
- **Storage:** 1TB NVMe SSD
- **Network:** 100 Mbps upload

### Optimal Specs (4+ Streams @ 1080p60)
- **CPU:** Ryzen 9 7950X / Intel i9-13900K (16+ cores)
- **GPU:** RTX 4080+
- **RAM:** 64GB
- **Storage:** 2TB NVMe SSD
- **Network:** 500 Mbps upload (dedicated streaming connection)

---

## When to Use MultiStream Director vs. Single OBS

### Use Single OBS + Multi-RTMP Plugin If:
- ✅ Same game, same settings
- ✅ Only difference is streaming platform
- ✅ Want simplest setup
- ✅ Limited hardware resources

**Recommended tools:**
- OBS + [Multiple Output Plugin](https://obsproject.com/forum/resources/multiple-rtmp-outputs-plugin.964/)
- [Restream.io](https://restream.io) (cloud service, $16/month)

### Use MultiStream Director If:
- ✅ Different games per platform
- ✅ Different accounts/settings per platform
- ✅ Different OBS scenes/overlays per stream
- ✅ Need complete isolation (cookies, saves, configs)
- ✅ Want independent control of each stream
- ✅ Need different quality settings per platform

---

## Simplified DIY Alternative (No Custom App)

If you don't want to build MultiStream Director, use existing tools:

### Required Tools:
1. **[Sandboxie Plus](https://sandboxie-plus.com/)** (free) - Run isolated game instances
2. **OBS Portable** x3 - Separate OBS folder per stream
3. **[AutoHotkey](https://www.autohotkey.com/)** - Control all instances with global hotkeys
4. **[Actual Multiple Monitors](https://www.actualtools.com/multiplemonitors/)** - Window management
5. **[VoiceMeeter Potato](https://vb-audio.com/Voicemeeter/potato.htm)** - Audio routing

### Setup Script:

Create `launch_multistream.bat`:

```batch
@echo off
echo Starting MultiStream Setup...

REM Launch Slot 1 (Twitch)
start /d "C:\Sandbox1\" falling-heads.exe --config twitch.cfg
timeout /t 2
start /d "C:\OBS1\" obs64.exe --profile twitch --startstreaming

REM Launch Slot 2 (YouTube)
start /d "C:\Sandbox2\" falling-heads.exe --config youtube.cfg
timeout /t 2
start /d "C:\OBS2\" obs64.exe --profile youtube --startstreaming

REM Launch Slot 3 (Kick)
start /d "C:\Sandbox3\" falling-heads.exe --config kick.cfg
timeout /t 2
start /d "C:\OBS3\" obs64.exe --profile kick --startstreaming

echo All streams launched!
pause
```

### AutoHotkey Control Script:

```autohotkey
; Global hotkey to stop all streams
^!q::  ; Ctrl+Alt+Q
{
    Run, taskkill /im obs64.exe /f
    Run, taskkill /im falling-heads.exe /f
    MsgBox, All streams stopped!
}

; Switch to Slot 1 (Twitch)
^!1::  ; Ctrl+Alt+1
{
    WinActivate, ahk_exe falling-heads.exe ahk_pid %Slot1PID%
}

; Switch to Slot 2 (YouTube)
^!2::  ; Ctrl+Alt+2
{
    WinActivate, ahk_exe falling-heads.exe ahk_pid %Slot2PID%
}

; Switch to Slot 3 (Kick)
^!3::  ; Ctrl+Alt+3
{
    WinActivate, ahk_exe falling-heads.exe ahk_pid %Slot3PID%
}
```

---

## Network Bandwidth Calculator

### Per Stream Requirements:

| Quality | Bitrate | Upload Speed Needed |
|---------|---------|---------------------|
| 720p30  | 2500 kbps | 4 Mbps |
| 720p60  | 4000 kbps | 6 Mbps |
| 1080p30 | 4500 kbps | 7 Mbps |
| 1080p60 | 6000 kbps | 9 Mbps |
| 1440p60 | 9000 kbps | 13 Mbps |

### Total Bandwidth for 3 Streams:

| Configuration | Total Upload Needed |
|---------------|---------------------|
| 3x 720p60 | 18 Mbps |
| 3x 1080p60 | 27 Mbps |
| 2x 1080p60 + 1x 720p60 | 24 Mbps |

**Recommendation:** Add 20% buffer for overhead → 3x 1080p60 = **35 Mbps upload minimum**

---

## Audio Routing Setup

Use **VoiceMeeter Potato** to route audio independently:

```
Input:
├─ Microphone → VoiceMeeter Input 1
├─ Game 1 Audio → VoiceMeeter Input 2 (Virtual Cable A)
├─ Game 2 Audio → VoiceMeeter Input 3 (Virtual Cable B)
└─ Game 3 Audio → VoiceMeeter Input 4 (Virtual Cable C)

Output:
├─ OBS 1 listens to: Mic + Game 1
├─ OBS 2 listens to: Mic + Game 2
└─ OBS 3 listens to: Mic + Game 3
```

---

## Game Integration Recommendations

### For falling-heads Game:

Add command-line config support:

```bash
# Launch for Twitch
falling-heads.exe --config twitch.toml --window-title "Falling Heads - Twitch"

# Launch for YouTube
falling-heads.exe --config youtube.toml --window-title "Falling Heads - YouTube"

# Launch for Kick
falling-heads.exe --config kick.toml --window-title "Falling Heads - Kick"
```

### Config File Example (`twitch.toml`):

```toml
[display]
width = 1920
height = 1080
fullscreen = false
window_x = 0
window_y = 0

[game]
fps_limit = 60
vsync = false
difficulty = "normal"

[stream]
platform = "twitch"
username = "streamer1"
show_overlay = true
```

---

## Future Feature Ideas

### Phase 1 (MVP)
- ✅ Basic slot management (start/stop/pause)
- ✅ Resource monitoring
- ✅ OBS integration via websocket
- ✅ Per-slot configuration

### Phase 2
- ✅ Unified chat dashboard
- ✅ Templates system
- ✅ Auto-quality adjustment
- ✅ Global hotkeys

### Phase 3
- ✅ AI-powered scene switching (detect game events)
- ✅ Cloud config sync
- ✅ Multi-PC streaming (distribute load)
- ✅ Analytics dashboard (viewer stats per platform)

### Phase 4 (Advanced)
- ✅ Automatic highlight detection
- ✅ Cross-platform raid/host management
- ✅ Sponsor integration (ads management)
- ✅ VOD auto-upload to YouTube

---

## Monetization Options (If Building as Product)

### Free Tier
- Up to 2 simultaneous streams
- Basic resource monitoring
- Community templates

### Pro Tier ($19.99/month)
- Up to 5 simultaneous streams
- Advanced analytics
- Cloud config sync
- Priority support

### Studio Tier ($49.99/month)
- Unlimited streams
- Multi-PC support
- White-label options
- API access

---

## Conclusion

MultiStream Director solves a real problem for content creators who want to maximize their reach across multiple platforms without the complexity of managing multiple setups manually.

**Key Value Propositions:**
1. **Time savings** - Set up once, stream everywhere
2. **Resource optimization** - Smart allocation prevents crashes
3. **Professional management** - Unified control interface
4. **Flexibility** - Different games, accounts, settings per platform

**Next Steps:**
1. Prototype with Tauri + Rust (matches your tech stack)
2. Start with 2-slot MVP
3. Test with falling-heads game
4. Gather feedback from streaming community
5. Expand features based on user needs

---

## Contact & Contributions

If you're interested in building this or want to collaborate:
- Open an issue on GitHub
- Join our Discord community
- Submit feature requests

**License:** MIT (if building as open-source)
**Tech Stack:** Rust + Tauri + React/Svelte
**Target Platforms:** Windows (primary), macOS/Linux (future)

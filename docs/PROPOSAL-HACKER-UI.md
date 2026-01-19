# Proposal: Hacker Terminal UI Effects

**Status:** In Progress  
**Author:** AI Assistant  
**Date:** January 2026

---

## What's Done (Phase 1)

- [x] Theme system with 4 themes: `hacker`, `matrix`, `crt`, `minimal`
- [x] `--theme` CLI flag
- [x] Bright green color scheme
- [x] Thick borders (`┏━━━┓`)
- [x] Lambda prompt (`λ `)
- [x] Block cursor (`█`)

---

## Remaining Effects

### Effect 1: Blinking Cursor

**What it does:** The block cursor (`█`) blinks on and off every 500ms, like a real terminal.

**Visual example:**
```
Frame 1:  λ hello world█        (cursor visible)
Frame 2:  λ hello world         (cursor hidden)
Frame 3:  λ hello world█        (cursor visible)
...repeats...
```

**Effort:** Low (~30 min)  
**Impact:** Makes it feel alive

---

### Effect 2: Typing Animation

**What it does:** When the dialog opens, text appears character-by-character like someone is typing it.

**Visual example:**
```
Time 0ms:    λ 
Time 50ms:   λ e
Time 100ms:  λ ex
Time 150ms:  λ exp
Time 200ms:  λ expl
Time 250ms:  λ expla
Time 300ms:  λ explain
```

**Where it applies:**
- Initial prompt text (if provided via CLI)
- Help text at bottom
- Placeholder labels

**User can skip:** Press any key to show all text immediately

**Effort:** Medium (~1-2 hours)  
**Impact:** Cool "hacker typing" feel

---

### Effect 3: Scanline

**What it does:** A single horizontal line moves slowly down the screen, simulating an old CRT monitor refresh.

**Visual example:**
```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ ░▒▓ OPENCODE ▓▒░         ┃  <-- scanline here (slightly brighter)
┃                          ┃
┃  λ hello█                ┃
┃                          ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━┛

Next frame:
┏━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ ░▒▓ OPENCODE ▓▒░         ┃
┃                          ┃  <-- scanline moved down
┃  λ hello█                ┃
┃                          ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

**Effort:** Medium (~1 hour)  
**Impact:** Subtle retro CRT feel

---

### Effect 4: Glitch on Focus Change

**What it does:** When you press Tab to switch between input/buttons, random characters briefly flash for ~100ms.

**Visual example:**
```
Before Tab:   SEND     CANCEL
During Tab:   $#@%     ░▒▓█▀▄   (100ms of random chars)
After Tab:    SEND     CANCEL   (back to normal, focus moved)
```

**Effort:** Medium (~1 hour)  
**Impact:** Cyberpunk "glitchy" feel

---

### Effect 5: Matrix Rain Background

**What it does:** Green characters fall down in the background behind the dialog, like in The Matrix movie.

**Visual example:**
```
    ア  カ     サ        タ
  イ      キ シ   ス  チ
    ウ  ク     セ    ツ
┏━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ ░▒▓ OPENCODE ▓▒░         ┃
┃                          ┃
┃  λ hello█                ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━┛
  エ    ケ  ソ      テ
    オ      コ        ト
```

Characters used: Katakana, numbers, symbols  
Speed: Each column falls at random speed

**Effort:** High (~3-4 hours)  
**Impact:** Very cool but might be distracting

---

### Effect 6: Boot Sequence

**What it does:** When app starts, shows a fake "computer booting" animation before the main UI.

**Visual example:**
```
BIOS v2.0.26 (c) 2026 OpenCode Systems
Memory Test: 65536K OK
Detecting drives...
  Primary: HELIX-SSD 256GB
Loading OPENCODE.SYS...
Initializing neural interface... [OK]
Establishing quantum link... [OK]

> Connection established
> Welcome to OpenCode

[then shows main UI]
```

Each line appears with a small delay (50-200ms)

**Effort:** Medium (~1-2 hours)  
**Impact:** Fun novelty, but adds startup delay

---

## Recommendations

| Effect | Recommended? | Reason |
|--------|--------------|--------|
| Blinking cursor | Yes | Simple, looks good |
| Typing animation | Maybe | Cool but might slow down workflow |
| Scanline | Maybe | Subtle, good for CRT theme |
| Glitch | No | Might be annoying |
| Matrix rain | No | Too distracting for a tool |
| Boot sequence | No | Adds unnecessary delay |

---

## CLI Flags (if implemented)

```bash
# Enable blinking cursor
opencode-helix --blink ask

# Disable all animations
opencode-helix --no-anim ask

# Enable boot sequence (one-time novelty)
opencode-helix --boot ask
```

---

## Next Steps

Tell me which effects you want and I'll implement them.

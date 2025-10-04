# API Testing Guide

## 🚀 External API is Live!

The game now has an HTTP API running on **http://localhost:3000**

## 🧨 Test TNT Power

**1. Start the game:**
```bash
cargo run
```

**2. In another terminal, send TNT command:**
```bash
curl -X POST http://localhost:3000/command -H "Content-Type: application/json" -d "{\"action\":\"TNT\"}"
```

**Expected Result:**
- Red TNT square spawns above hero
- After 3 seconds, it explodes
- All blocks in 150-pixel radius are destroyed!

## 📡 Available Commands

| Command | Description | Status |
|---------|-------------|--------|
| `TNT` | Spawns explosive TNT above hero (150px radius) | ✅ Working |
| `FAST` | Speed up hero x2 for 5 seconds | ✅ Working |
| `SLOW` | Slow down hero x0.5 for 5 seconds | ✅ Working |
| `LASER` | Fire laser beam downward (5 blocks wide) | ✅ Working |
| `ARROW` | Circular arrow barrage (12 arrows) | ✅ Working |
| `BOLT` | Lightning strike | ⏳ Coming next |
| `GHAST` | Spawn shooting enemy | ⏳ Coming next |
| `BOX` | Spawn loot box | ⏳ Coming next |
| `MAGNET` | Attract resources | ⏳ Coming next |
| `FUSS` | Chaos mode | ⏳ Coming next |

## 🔥 Test Examples

### TNT (Explosive)
```bash
curl -X POST http://localhost:3000/command \
  -H "Content-Type: application/json" \
  -d '{"action":"TNT"}'
```

### FAST (Speed Boost)
```bash
curl -X POST http://localhost:3000/command \
  -H "Content-Type: application/json" \
  -d '{"action":"FAST"}'
```

### SLOW (Slow Motion)
```bash
curl -X POST http://localhost:3000/command \
  -H "Content-Type: application/json" \
  -d '{"action":"SLOW"}'
```

### LASER (Beam Attack)
```bash
curl -X POST http://localhost:3000/command \
  -H "Content-Type: application/json" \
  -d '{"action":"LASER"}'
```

### ARROW (Circular Barrage)
```bash
curl -X POST http://localhost:3000/command \
  -H "Content-Type: application/json" \
  -d '{"action":"ARROW"}'
```

### Multiple TNT (spam it!)
```bash
for i in {1..5}; do
  curl -X POST http://localhost:3000/command \
    -H "Content-Type: application/json" \
    -d '{"action":"TNT"}'
  sleep 0.5
done
```

### Power Combo (Try multiple powers!)
```bash
curl -X POST http://localhost:3000/command -H "Content-Type: application/json" -d '{"action":"FAST"}'
sleep 1
curl -X POST http://localhost:3000/command -H "Content-Type: application/json" -d '{"action":"LASER"}'
sleep 1
curl -X POST http://localhost:3000/command -H "Content-Type: application/json" -d '{"action":"ARROW"}'
sleep 1
curl -X POST http://localhost:3000/command -H "Content-Type: application/json" -d '{"action":"TNT"}'
```

### Invalid Command (test error handling)
```bash
curl -X POST http://localhost:3000/command \
  -H "Content-Type: application/json" \
  -d '{"action":"INVALID"}'
```

## ✅ Success Response
```json
{
  "success": true,
  "message": "Power 'TNT' queued!"
}
```

## ❌ Error Response
```json
{
  "success": false,
  "message": "Unknown command: INVALID. Available: TNT, FAST, SLOW, LASER, ARROW, BOLT, GHAST, BOX, MAGNET, FUSS"
}
```

## 🎮 Integration Ideas

### Twitch Chat Bot
```python
import requests

def trigger_power(power_name):
    response = requests.post(
        "http://localhost:3000/command",
        json={"action": power_name}
    )
    return response.json()

# When viewer types "!tnt" in chat:
trigger_power("TNT")
```

### Discord Bot
```javascript
client.on('messageCreate', message => {
  if (message.content === '!tnt') {
    fetch('http://localhost:3000/command', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action: 'TNT' })
    });
  }
});
```

### Web Dashboard
Create a simple HTML page with buttons:
```html
<button onclick="triggerPower('TNT')">🧨 TNT</button>
<button onclick="triggerPower('LASER')">⚡ LASER</button>

<script>
function triggerPower(action) {
  fetch('http://localhost:3000/command', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ action })
  });
}
</script>
```

## 🐛 Troubleshooting

**API not responding:**
- Check if game is running
- Check console for "API Server running on http://127.0.0.1:3000"
- Try: `netstat -ano | findstr :3000` (Windows)

**TNT not spawning:**
- Make sure you're in-game (not in menu)
- Check game console for "🧨 TNT power activated!"
- Try moving the hero first

**Port 3000 already in use:**
- Close other apps using port 3000
- Or modify the port in `external_api_plugin.rs`

---

## 💡 Power Details

### TNT 🧨
- Spawns a red TNT block 50 pixels above the hero
- 3-second countdown timer
- Destroys all blocks within 150-pixel radius
- Great for clearing large areas!

### FAST ⚡
- Multiplies hero movement speed by 2x
- Lasts for 5 seconds
- Stack multiple FAST commands for even more speed!

### SLOW 🐌
- Reduces hero movement speed to 0.5x
- Lasts for 5 seconds
- Can be used to carefully navigate tight spaces

### LASER 💜
- Fires a purple beam downward from hero
- 320 pixels wide (5 blocks)
- Lasts 0.5 seconds
- Destroys all blocks in its path
- Perfect for clearing a vertical column!

### ARROW 🏹
- Spawns 12 arrows in a circular pattern
- Arrows fly outward in all directions
- Each arrow destroys the first block it hits
- Great for clearing blocks around the hero!

---

**Next:** We'll add BOLT, GHAST, BOX, MAGNET, and FUSS powers!

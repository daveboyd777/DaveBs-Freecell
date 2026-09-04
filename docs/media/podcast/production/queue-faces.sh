#!/bin/bash
cd "$(dirname "$0")"
API="https://api.x.ai/v1/videos/generations"
AUTH="Authorization: Bearer $XAI_API_KEY"

queue_face() { # name photo prompt
  local name="$1"; local photo="$2"; local prompt="$3"
  base64 -w0 "$photo" > "work/${name}.b64"
  cat > "work/${name}.json" <<EOF
{
  "model": "grok-imagine-video-1.5",
  "prompt": "$prompt",
  "image": {"url": "data:image/jpeg;base64,$(cat work/${name}.b64)"},
  "duration": 10
}
EOF
  local rid=$(curl -s --max-time 180 -X POST "$API" -H "$AUTH" -H "Content-Type: application/json" -d "@work/${name}.json" | grep -o '"request_id":"[^"]*"' | cut -d'"' -f4)
  echo "$name $rid"
  rm -f "work/${name}.b64"
}

queue_face nick_a work/dave_1280.jpg "The man in this photo comes alive and speaks expressively to someone off camera, natural subtle head movements, occasional small hand gesture, slight knowing smiles, eyebrows animate. Preserve his exact face and likeness. 1930s black and white film footage, film grain, soft key lighting, static camera"

queue_face nick_b work/dave_1280.jpg "The man in this photo talks and gestures mid-conversation, leaning slightly forward as he explains something with dry wit, small nodding motions, a wry smile. Preserve his exact face and likeness. 1930s black and white film footage, film grain, soft lamplight, static camera"

queue_face nora_a work/deb_1280.jpg "The woman in this photo comes alive and speaks warmly to someone off camera, curious engaged expression, natural subtle head tilts and small nods, animated eyebrows, a warm smile. Preserve her exact face and likeness. 1930s black and white film footage, film grain, soft key lighting, static camera"

queue_face nora_b work/deb_1280.jpg "The woman in this photo laughs lightly and speaks teasingly to someone off camera, playful knowing expression, small head tilt, natural gestures. Preserve her exact face and likeness. 1930s black and white film footage, film grain, soft lamplight, static camera"

#!/bin/bash
# Queue all remaining video generations on xAI grok-imagine-video-1.5
cd "$(dirname "$0")"
API="https://api.x.ai/v1/videos/generations"
AUTH="Authorization: Bearer $XAI_API_KEY"
CT="Content-Type: application/json"

DAVE_B64=$(base64 -w0 work/dave_1280.jpg)
DEB_B64=$(base64 -w0 work/deb_1280.jpg)

queue() { # name, json
  local name="$1"; local json="$2"
  local rid=$(curl -s --max-time 120 -X POST "$API" -H "$AUTH" -H "$CT" -d "$json" | grep -o '"request_id":"[^"]*"' | cut -d'"' -f4)
  echo "$name $rid"
}

queue asta2 '{
  "model": "grok-imagine-video-1.5",
  "prompt": "A wire fox terrier dozing by the leg of a card table suddenly perks its ears up and raises its head with curiosity, holds a moment, then settles back down to sleep. 1930s black and white film footage, film grain, soft warm lamplight, static camera",
  "duration": 6
}'

queue asta3 '{
  "model": "grok-imagine-video-1.5",
  "prompt": "A wire fox terrier lying by a card table sits up smartly, tail wagging with delight, alert and happy expression, ears up. 1930s black and white film footage, film grain, warm indoor lamplight, static camera",
  "duration": 6
}'

queue establish '{
  "model": "grok-imagine-video-1.5",
  "prompt": "Slow cinematic establishing shot of a 1930s Manhattan apartment study at night: a card table with a game of FreeCell solitaire laid out, a crystal decanter and two glasses, a wire fox terrier curled asleep by the table leg, warm lamp glow, cigarette smoke drifting in the light. Black and white film noir cinematography, film grain, gentle slow push in",
  "duration": 8
}'

queue nick_a "{
  \"model\": \"grok-imagine-video-1.5\",
  \"prompt\": \"The man in this photo comes alive and speaks expressively to someone off camera, natural subtle head movements, occasional small hand gesture, slight knowing smiles, eyebrows animate. Preserve his exact face and likeness. 1930s black and white film footage, film grain, soft key lighting, static camera\",
  \"image\": {\"url\": \"data:image/jpeg;base64,$DAVE_B64\"},
  \"duration\": 10
}"

queue nick_b "{
  \"model\": \"grok-imagine-video-1.5\",
  \"prompt\": \"The man in this photo talks and gestures mid-conversation, leaning slightly forward as he explains something with dry wit, small nodding motions, a wry smile. Preserve his exact face and likeness. 1930s black and white film footage, film grain, soft lamplight, static camera\",
  \"image\": {\"url\": \"data:image/jpeg;base64,$DAVE_B64\"},
  \"duration\": 10
}"

queue nora_a "{
  \"model\": \"grok-imagine-video-1.5\",
  \"prompt\": \"The woman in this photo comes alive and speaks warmly to someone off camera, curious engaged expression, natural subtle head tilts and small nods, animated eyebrows, a warm smile. Preserve her exact face and likeness. 1930s black and white film footage, film grain, soft key lighting, static camera\",
  \"image\": {\"url\": \"data:image/jpeg;base64,$DEB_B64\"},
  \"duration\": 10
}"

queue nora_b "{
  \"model\": \"grok-imagine-video-1.5\",
  \"prompt\": \"The woman in this photo laughs lightly and speaks teasingly to someone off camera, playful knowing expression, small head tilt, natural gestures. Preserve her exact face and likeness. 1930s black and white film footage, film grain, soft lamplight, static camera\",
  \"image\": {\"url\": \"data:image/jpeg;base64,$DEB_B64\"},
  \"duration\": 10
}"

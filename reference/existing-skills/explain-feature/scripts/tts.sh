#!/usr/bin/env bash
# Free, offline text-to-speech for the narration script.
#   macOS: built-in `say` (best installed voice for the language) -> .m4a
#   Linux: Piper (https://github.com/rhasspy/piper) if installed + a voice model -> .wav (.mp3 if ffmpeg)
# Prints the generated file path on success. Exits 2 when no engine is available
# (the HTML report then falls back to the browser's built-in speech).
#
# Usage: tts.sh <narration.txt> <output-path-without-extension> <fr|en>
# Overrides: EXPLAIN_TTS_VOICE (macOS voice name), PIPER_MODEL_FR / PIPER_MODEL_EN (path to .onnx)
set -euo pipefail

in="$1"; out="$2"; lang="${3:-fr}"

if [ "$(uname)" = "Darwin" ] && command -v say >/dev/null; then
  locale_re=$([ "$lang" = "fr" ] && echo "fr_FR" || echo "en_(US|GB)")
  voice="${EXPLAIN_TTS_VOICE:-}"
  if [ -z "$voice" ]; then
    voices="$(say -v '?' | sed -nE "s/^(.+[^ ]) +(${locale_re}) +#.*/\1/p")"
    voice="$(printf '%s\n' "$voices" | grep -iE 'premium' | head -1 || true)"
    [ -z "$voice" ] && voice="$(printf '%s\n' "$voices" | grep -iE 'enhanced|amélioré' | head -1 || true)"
    [ -z "$voice" ] && voice="$(printf '%s\n' "$voices" | grep -xE 'Thomas|Samantha|Daniel' | head -1 || true)"
    [ -z "$voice" ] && voice="$(printf '%s\n' "$voices" | grep -viE 'grandma|grandpa|rocko|shelley|sandy|eddy|reed|flo' | head -1 || true)"
  fi
  args=(); [ -n "$voice" ] && args=(-v "$voice")
  say "${args[@]}" -f "$in" -o "$out.aiff"
  afconvert -f m4af -d aac "$out.aiff" "$out.m4a" && rm -f "$out.aiff"
  echo "voice: ${voice:-system default}" >&2
  echo "$out.m4a"
  exit 0
fi

if command -v piper >/dev/null; then
  model_var="PIPER_MODEL_$(echo "$lang" | tr a-z A-Z)"
  model="${!model_var:-}"
  [ -z "$model" ] && model="$(ls "$HOME"/.local/share/piper/${lang}_*.onnx 2>/dev/null | head -1 || true)"
  if [ -n "$model" ] && [ -f "$model" ]; then
    piper --model "$model" --output_file "$out.wav" < "$in" >/dev/null 2>&1
    if command -v ffmpeg >/dev/null; then
      ffmpeg -loglevel error -y -i "$out.wav" -codec:a libmp3lame -q:a 4 "$out.mp3" && rm -f "$out.wav"
      echo "$out.mp3"
    else
      echo "$out.wav"
    fi
    exit 0
  fi
  echo "piper found but no ${lang} voice model (set $model_var or put one in ~/.local/share/piper/)" >&2
fi

echo "no offline TTS engine available" >&2
exit 2

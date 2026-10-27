# Provider-Kompatibilität (Modelle)

> Getrennt von der Protect-Matrix: Diese Tabelle belegt nur, dass AI-Keyless mit einem
> Modellserver sprechen kann und ein Modell ein **synthetisches** Testbild versteht. Sie sagt
> nichts über Protect oder die Qualität bei echten Kamerabildern aus.

## Prüfverfahren

- **Verbindung:** `GET /api/tags` (Ollama) bzw. `GET /models` (OpenAI-kompatibel).
- **Bildtest:** eigenes Testbild `crates/providers/assets/synthetic-red-64.png` (64×64,
  einfarbig rot) mit dem Prompt „Which single color fills this image? Answer with one English
  word.“ Bestanden, wenn die Antwort das Wort `red` enthält.
- Ergebnisse werden pro Provider in `provider_tests` gespeichert und in der UI angezeigt.

## Ergebnisse

| Datum | Server | Modell | Hardware | Ergebnis | Latenz | Stufe |
|---|---|---|---|---|---|---|
| 2026-10-26 | Mock (Ollama-/OpenAI-Protokoll) | – | – | Format, Fehlerfälle, SSRF-Schutz | – | `synthetic_verified` |
| 2026-10-26 | Ollama 0.12.6 (Docker, `ollama/ollama:0.12.6`) | `moondream` (latest) | CPU, Cloud-Container | ✗ leere Antwort bei `/api/chat` und `/api/generate`, auch bei 384×384 | 0,1–16 s | `unsupported` (mit dieser Ollama-Version) |
| 2026-10-26 | Ollama 0.12.6 (Docker) | `granite3.2-vision` (latest, 2B) | CPU, Cloud-Container | ✓ „Red“ (64×64 und 384×384), über UI und API | Kaltstart 31,7 s, warm 0,4–4,7 s | getestet (synthetisch) |

| 2026-10-27 | Ollama 0.12.6 (Lab-Overlay, Docker Desktop/WSL2) | `granite3.2-vision` (latest, 2B, `num_ctx` 16384 aus Modelfile) | NVIDIA T600 (4 GB, 2,7 GiB frei), teilweise ausgelagert (18/41 Schichten GPU) | ✓ „Red“ (64×64), über UI | erster Aufruf nach Laden 27 s bzw. >120 s (Zeitlimit, mit Flash-Attention + q8_0-KV), danach 458 ms | getestet (synthetisch); Kaltstartursache offen |

Noch offen: Messung auf der Zielhardware (NVIDIA < 12 GB, Intel/AMD; Q-014), OpenAI-kompatible
Server (vLLM, LM Studio, llama.cpp) mit echtem Modell, Qualitätsbewertung mit realistischen,
aber künstlichen Szenen (AKL-M060-04).

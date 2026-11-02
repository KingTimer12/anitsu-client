# Player de músicas: plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** abrir uma pasta de músicas da nuvem como um álbum no estilo "encarte de CD japonês" e tocar as faixas pelo Rust, sem perda de qualidade e sem gravar áudio em disco. Uma barra fixa continua tocando enquanto se navega pelo app.

**Architecture:**
- **Rust** (`src-tauri/src/music/`):
  - lê cada faixa da nuvem com **um** `Range: bytes=N-` em stream para um buffer em RAM;
  - decodifica com symphonia e reamostra com rubato só quando a taxa da faixa difere da do dispositivo;
  - aplica volume e ganho de álbum em `f32` e toca via cpal (WASAPI);
  - emite `music://state` para o JS.
- **SolidJS:**
  - monta o álbum a partir da listagem (lógica pura em `src/lib/music/logic.ts`);
  - mostra `AlbumView` (encarte) e `MusicBar` (fixa embaixo);
  - só envia comandos ao Rust.

**Tech Stack:**
- Rust: `symphonia` 0.5 (features `all-codecs`, `all-formats`), `cpal` 0.15, `rubato` 0.16, `ebur128` 0.1, `souvlaki` 0.8; reqwest via `tauri-plugin-http`, usando `Response::chunk()`.
- TS: SolidJS, Tailwind v4, lucide-solid, vitest.

**Spec:** `docs/superpowers/specs/2026-11-02-player-musica-design.md`

## Global Constraints

- **Áudio nunca vai para o disco.** Só números vão para o disco: loudness e metadados, em `app_data_dir/music-cache.json`.
- **Tokens só no Rust.** O JS envia caminho e tamanho; quem busca é o Rust, via `apis.nuvem.send_raw(&auth, Method::GET, "preview", |r| r.query(&[("path", path)]).header(RANGE, …))`, como em `src-tauri/src/reader/mod.rs` (`fetch_range`).
- **Um pedido aberto por faixa:** `Range: bytes=N-`. Um seek para além do que já chegou cancela o pedido atual e abre outro a partir do ponto pedido; o que já chegou fica.
- **429:** esperar `Retry-After` (em segundos; padrão 5 s) e tentar de novo, no máximo 5 vezes. Não interromper o áudio já em buffer.
- **Metadados em segundo plano:** uma faixa por vez, lendo só `0..=262143` (256 KiB) do arquivo.
- **Extensões de áudio:** `.flac .mp3 .wav .ogg .m4a .opus`. Qualquer outra (`.ape`, `.cue`, `.wv`…) aparece em "Arquivos" e só pode ser baixada.
- **Ganho:**
  - fixo por faixa e decidido antes de ela tocar;
  - ordem: tag de álbum (`REPLAYGAIN_ALBUM_GAIN`), tag de faixa (`REPLAYGAIN_TRACK_GAIN`), média de loudness das faixas já medidas do álbum, 0 dB;
  - alvo **-18 LUFS**;
  - o ganho é limitado para que `pico × ganho_linear ≤ 1.0`.
- **Reamostragem:** só quando a taxa da faixa ≠ taxa do dispositivo. `rubato::SincFixedIn` com `sinc_len 256`, `f_cutoff 0.95`, `oversampling_factor 256`, `BlackmanHarris2`. Taxas iguais: amostras passam intactas, multiplicadas apenas pelo ganho total.
- **Visual:**
  - fonte `--font-mincho` nos números e no título;
  - `--color-brand` (#e8283c) no obi e no fio; `--color-brand-ink` (#ff7a85) na faixa atual;
  - padrão de retícula `screentone` do leitor;
  - ícones `lucide-solid`.
- **Textos em pt-BR.**
- **Commits sem trailer de atribuição** (sem Co-Authored-By).
- **Abrir o player de vídeo pausa a música.** Abrir o leitor de mangá **não** pausa.
- Progresso e preferências no `localStorage`, chave `higanbana:music:v1`.

## Review Focus

1. **Seek antes do fim do download.** Deve abrir um novo Range sem travar a UI e sem tocar lixo. Teste: `source::tests::seek_past_received_restarts_fetch_at_offset` (Task 2).
2. **Faixas com taxas diferentes no mesmo álbum** (44.1 → 48 kHz, ou WAV 96 kHz). Não pode haver chiado, nem velocidade ou tom errado. Teste: `decode::tests::resample_keeps_duration` (Task 3), mais a verificação manual na Task 6.
3. **Álbum só com APE+CUE**, ou com zero faixas tocáveis. Não vira encarte vazio; a pasta fica como lista normal. Teste: `buildAlbum returns null without playable tracks` (Task 1).
4. **Ganho alto + pico alto** não podem clipar. Teste: `gain::tests::gain_is_limited_by_peak` (Task 4).
5. **Nomes estranhos:** `1-01 - X.flac`, `01. X.mp3`, `Track 3.wav`, japonês, sem número. A ordem natural e os discos precisam estar certos. Teste: `parseTrack handles disc-track, dotted and bare names` (Task 1).

---

### Task 1: Lógica do álbum (TS puro)

**Files:**
- Create: `src/lib/music/logic.ts`
- Test: `src/lib/music/logic.test.ts`

**Interfaces:**
- Consumes: `FileEntry` de `src/lib/anitsu`.
- Produces:
  ```ts
  export const AUDIO_EXT: Set<string>; // ".flac",".mp3",".wav",".ogg",".m4a",".opus"
  export const isAudio: (f: FileEntry) => boolean;
  export interface TrackName { disc: number | null; number: number | null; title: string }
  export function parseTrack(fileName: string): TrackName;
  export interface Track { path: string; size: number; name: string; disc: number; number: number | null; title: string; ext: string }
  export interface Album { path: string; title: string; catalog: string | null; year: string | null; tracks: Track[]; discs: number[]; coverPath: string | null; others: FileEntry[]; discDirs: string[] }
  export function catalogCode(folder: string): string | null;
  export function albumYear(folder: string): string | null;
  export function albumTitle(folder: string): string;
  export function isDiscDir(name: string): number | null; // "Disc 1"/"CD2"/"Disco 2" → 1/2/2
  export function pickCover(dir: string, files: FileEntry[], sub?: { dir: string; files: FileEntry[] }[]): string | null;
  export function buildAlbum(path: string, files: FileEntry[], discs?: { n: number; dir: string; files: FileEntry[] }[]): Album | null;
  export const formatDuration: (secs: number) => string; // 271 → "4:31", 3725 → "1:02:05"
  ```

- [ ] **Step 1: Write the failing tests**

```ts
test("parseTrack handles disc-track, dotted and bare names", () => {
  expect(parseTrack("1-03 - Action BGM (Thriller).flac")).toEqual({ disc: 1, number: 3, title: "Action BGM (Thriller)" });
  expect(parseTrack("01 - Morning Arch.flac")).toEqual({ disc: null, number: 1, title: "Morning Arch" });
  expect(parseTrack("01. Kimi wo Nosete.mp3")).toEqual({ disc: null, number: 1, title: "Kimi wo Nosete" });
  expect(parseTrack("Track 3.wav")).toEqual({ disc: null, number: 3, title: "Track 3" });
  expect(parseTrack("Candy Candy.MP3")).toEqual({ disc: null, number: null, title: "Candy Candy" });
  expect(parseTrack("1984-01-25 LOVEさりげなく.wav").number).toBeNull(); // data não é número de faixa
});
test("catalogCode, year and title from folder names", () => {
  expect(catalogCode("2011.05.25 - A-Channel TV1 OP1 - Morning Arch (SVWC-7767)")).toBe("SVWC-7767");
  expect(catalogCode("1990.01.01 - Nostalgic TV Anime Theme Songs Vol 2 (FDLA-6235)")).toBe("FDLA-6235");
  expect(catalogCode("All That Jazz - Ghibli Jazz")).toBeNull();
  expect(albumYear("2011.05.25 - A-Channel TV1 OP1 - Morning Arch (SVWC-7767)")).toBe("2011");
  expect(albumYear("[Album] V.A. - Studio Ghibli no Uta [2008.11.26][APE]")).toBe("2008");
  expect(albumTitle("2011.05.25 - A-Channel TV1 OP1 - Morning Arch (SVWC-7767)")).toBe("A-Channel TV1 OP1 - Morning Arch");
});
test("isDiscDir", () => {
  expect(isDiscDir("Disc 1")).toBe(1); expect(isDiscDir("CD2")).toBe(2); expect(isDiscDir("disco 3")).toBe(3); expect(isDiscDir("Scans")).toBeNull();
});
// f(name, size=1) cria FileEntry de arquivo; d(name) cria pasta.
test("pickCover: named cover anywhere > first root image > first sub image > null", () => {
  expect(pickCover("A", [f("02.jpg"), f("01.jpg")], [{ dir: "A/Scans", files: [f("Front.JPG")] }])).toBe("A/Scans/Front.JPG");
  expect(pickCover("A", [f("folder.png"), f("01.jpg")])).toBe("A/folder.png");
  expect(pickCover("A", [f("10.jpg"), f("2.jpg")])).toBe("A/2.jpg");
  expect(pickCover("A", [], [{ dir: "A/Scans", files: [f("b.jpg"), f("a.jpg")] }])).toBe("A/Scans/a.jpg");
  expect(pickCover("A", [f("01.flac")])).toBeNull();
});
test("buildAlbum returns null without playable tracks", () => {
  expect(buildAlbum("x", [f("Disc 1.ape"), f("Disc 1.cue")])).toBeNull();
});
test("buildAlbum sorts naturally, groups discs from names and from disc dirs, keeps others", () => {
  const a = buildAlbum("OSTs/X/2009.11.18 - OST (AVCA-29415)", [f("1-10 - J.flac"), f("2-01 - K.flac"), f("1-02 - B.flac"), f("1-01 - A.flac"), f("x.cue"), d("Scans")])!;
  expect(a.tracks.map((t) => t.title)).toEqual(["A", "B", "J", "K"]);
  expect(a.tracks.map((t) => t.disc)).toEqual([1, 1, 1, 2]);
  expect(a.discs).toEqual([1, 2]);
  expect(a.catalog).toBe("AVCA-29415");
  expect(a.others.map((o) => o.name)).toEqual(["Scans", "x.cue"]);
  const b = buildAlbum("Y", [d("CD1"), d("CD2")], [{ n: 2, dir: "Y/CD2", files: [f("01 - C.mp3")] }, { n: 1, dir: "Y/CD1", files: [f("01 - A.mp3"), f("02 - B.mp3")] }])!;
  expect(b.tracks.map((t) => [t.disc, t.title, t.path])).toEqual([[1, "A", "Y/CD1/01 - A.mp3"], [1, "B", "Y/CD1/02 - B.mp3"], [2, "C", "Y/CD2/01 - C.mp3"]]);
  expect(b.discDirs).toEqual(["Y/CD1", "Y/CD2"]);
  const c = buildAlbum("Z", [f("01 - A.flac")])!;
  expect(c.discs).toEqual([1]); // disco único = 1, sem separador na UI
});
test("formatDuration", () => { expect(formatDuration(271)).toBe("4:31"); expect(formatDuration(3725)).toBe("1:02:05"); });
```

- [ ] **Step 2: Run** `bun run test src/lib/music` → **Expected:** FAIL (module not found).
- [ ] **Step 3: Implement** in `src/lib/music/logic.ts`.
  - Natural order: `localeCompare(…, "pt-BR", { numeric: true, sensitivity: "base" })`, as in `src/lib/reader/logic.ts`.
  - Number regex: `^(?:(\d{1,2})-)?(\d{1,3})(?:\s*[-.]\s+|\.\s*|\s+-\s+)`, applied only when the number is not followed by `-\d\d-` (a date).
  - Bare `Track N`: number N, and the title stays the name.
- [ ] **Step 4: Run** `bun run test src/lib/music` → **Expected:** PASS.
- [ ] **Step 5: Commit** `feat(musica): lógica do álbum (faixas, discos, código, capa)`

---

### Task 2: Fonte por Range em RAM (Rust)

**Files:**
- Modify: `src-tauri/Cargo.toml` (add `symphonia = { version = "0.5", features = ["all-codecs", "all-formats"] }`)
- Create: `src-tauri/src/music/mod.rs` (only `pub mod source;` for now, plus `mod music;` in `lib.rs`)
- Create: `src-tauri/src/music/source.rs`

**Interfaces:**
- Produces:
  ```rust
  /// Origem dos bytes; produção = nuvem, testes = memória.
  pub trait Fetcher: Send + Sync + 'static {
      /// Abre um stream a partir de `offset`; chama `sink(bytes)` a cada pedaço até o fim,
      /// ou até `sink` devolver `false` (cancelado). Erro 429 já tratado aqui.
      fn stream(&self, offset: u64, sink: &mut dyn FnMut(&[u8]) -> bool) -> Result<(), String>;
  }
  pub struct RangeSource { /* Arc<Shared{ data: Mutex<Buf>, cv: Condvar }>, pos: u64, len: u64 */ }
  impl RangeSource { pub fn new(fetcher: Arc<dyn Fetcher>, len: u64) -> Self; pub fn received(&self) -> Vec<(u64, u64)>; }
  impl std::io::Read for RangeSource; impl std::io::Seek for RangeSource;
  impl symphonia::core::io::MediaSource for RangeSource; // is_seekable true, byte_len Some(len)
  pub struct CloudFetcher { app: AppHandle, path: String } // impl Fetcher via send_raw + resp.chunk(), runs on tauri::async_runtime::block_on in a std thread
  ```
- `Buf`:
  - a `Vec<u8>` of `len` bytes, allocated lazily, plus the list of received intervals;
  - the fetch runs on its own std thread;
  - `read` blocks on the `Condvar` until the requested byte arrives;
  - a seek outside the current stream's reach (more than 512 KiB ahead of the front, or before its start, into a hole) signals cancellation and opens a new stream at that offset.

- [ ] **Step 1: Write failing tests** with a `MemFetcher { data: Vec<u8>, chunk: usize, calls: Mutex<Vec<u64>> }`:
  - `reads_whole_file_in_order`: `read_to_end` == data, `calls == [0]`;
  - `seek_past_received_restarts_fetch_at_offset`: data of 4 MiB and a slow fetcher (sleeps 1 ms per 16 KiB); `seek(3 MiB)` before it arrives, then `read` of 4 bytes == `data[3MiB..3MiB+4]`, and `calls` contains `3 * 1024 * 1024`;
  - `seek_back_into_received_does_not_refetch`: after `read_to_end`, `seek(10)` + `read` → no new call;
  - `fetch_error_surfaces_as_io_error`: a fetcher that returns `Err` → `read` returns `Err` (it does not hang).
- [ ] **Step 2: Run** `cargo test --lib music::source` → **Expected:** FAIL (does not compile).
- [ ] **Step 3: Implement** `RangeSource` and `CloudFetcher`.
  - `CloudFetcher::stream`:
    - status 429: sleep `Retry-After`/5 s, up to 5 times;
    - status other than 200/206: `Err`;
    - on a 200 response, skip `offset` bytes.
- [ ] **Step 4: Run** `cargo test --lib music::source` → **Expected:** PASS. Also run `cargo clippy --all-targets` → 0 warnings.
- [ ] **Step 5: Commit** `feat(musica): fonte de áudio por Range em memória`

---

### Task 3: Decodificação, tags e reamostragem (Rust)

**Files:**
- Modify: `Cargo.toml` (add `rubato = "0.16"`)
- Create: `src-tauri/src/music/decode.rs`

**Interfaces:**
- Consumes: `RangeSource` (Task 2). The tests use `std::io::Cursor<Vec<u8>>` wrapped in `ReadOnlySource`/`MediaSourceStream`.
- Produces:
  ```rust
  pub struct TrackInfo { pub codec: String /* "FLAC","MP3","WAV","AAC","Vorbis","ALAC","Opus" */, pub sample_rate: u32, pub bits: Option<u32>, pub channels: u16, pub duration: Option<f64>, pub lossless: bool, pub tags: Tags }
  pub struct Tags { pub title: Option<String>, pub artist: Option<String>, pub album: Option<String>, pub track_gain: Option<f32>, pub album_gain: Option<f32>, pub track_peak: Option<f32>, pub album_peak: Option<f32>, pub cover: Option<(String /*mime*/, Vec<u8>)> }
  pub struct Decoder { /* format reader + decoder + track id */ }
  impl Decoder {
      pub fn open(src: Box<dyn MediaSource>, ext_hint: &str) -> Result<(Self, TrackInfo), String>;
      /// Próximo bloco intercalado em f32, no canal/taxa originais; None no fim.
      pub fn next_block(&mut self) -> Result<Option<Vec<f32>>, String>;
      pub fn seek(&mut self, secs: f64) -> Result<(), String>;
  }
  /// Só para metadados (Task 7): lê tags e duração de um prefixo do arquivo + tamanho total.
  pub fn probe_prefix(prefix: Vec<u8>, total_len: u64, ext_hint: &str) -> Result<TrackInfo, String>;
  pub struct Resampler { /* rubato SincFixedIn */ }
  impl Resampler { pub fn new(from: u32, to: u32, channels: usize) -> Self; pub fn process(&mut self, interleaved: &[f32]) -> Vec<f32>; pub fn flush(&mut self) -> Vec<f32>; }
  ```
- Notes on the tags:
  - Read from the container and from `metadata()` (ID3, Vorbis comments, MP4).
  - Gain format: `"-6.48 dB"` → `-6.48`.
- `probe_prefix` duration:
  - from `n_frames / sample_rate` when available;
  - for MP3 without Xing: `(total_len - id3_size) * 8 / bitrate`.

- [ ] **Step 1: Write failing tests.** A helper `wav(rate, channels, secs, freq) -> Vec<u8>` writes PCM 16-bit WAV in memory.
  - `decodes_wav_samples_and_info`: 1 s at 44100 stereo → total samples == 88200 (± one block), `info.codec == "WAV"`, `bits == Some(16)`, `lossless`;
  - `seek_lands_near_target`: after `seek(0.5)`, the samples left ≈ 0.5 s (±2 %);
  - `resample_keeps_duration`: 1 s at 44100 → `Resampler::new(44100, 48000, 2)`, process + flush → 96000 samples, ±1 %;
  - `probe_prefix_reads_wav_duration`: prefix of the first 4 KiB of a 2 s WAV plus `total_len` → `duration ≈ 2.0`;
  - `parses_replaygain_strings`: `parse_gain("-6.48 dB") == Some(-6.48)`, `parse_gain("+1.2 dB") == Some(1.2)`, `parse_gain("x") == None`.
- [ ] **Step 2: Run** `cargo test --lib music::decode` → **Expected:** FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → **Expected:** PASS; clippy clean.
- [ ] **Step 5: Commit** `feat(musica): decodificação com symphonia e reamostragem com rubato`

---

### Task 4: Ganho de álbum e cache de loudness (Rust)

**Files:**
- Modify: `Cargo.toml` (add `ebur128 = "0.1"`)
- Create: `src-tauri/src/music/gain.rs`

**Interfaces:**
- Consumes: `Tags` (Task 3).
- Produces:
  ```rust
  pub const TARGET_LUFS: f64 = -18.0;
  #[derive(Serialize, Deserialize, Default)] pub struct LoudnessCache { pub tracks: HashMap<String /*"path|size"*/, Measured> }
  #[derive(Serialize, Deserialize, Clone, Copy)] pub struct Measured { pub lufs: f64, pub peak: f32 }
  impl LoudnessCache { pub fn load(path: &Path) -> Self; pub fn save(&self, path: &Path); pub fn album_lufs(&self, album_keys: &[String]) -> Option<f64> /* média das medidas */; pub fn album_peak(&self, album_keys: &[String]) -> Option<f32>; }
  /// Ganho linear para uma faixa: tags álbum → faixa → média medida → 1.0; limitado por pico.
  pub fn track_gain(tags: &Tags, measured_album: Option<(f64, f32)>, enabled: bool) -> f32;
  /// Mede durante a reprodução: alimentado com os blocos tocados.
  pub struct Meter { /* ebur128::EbuR128 mode I | TRUE_PEAK? use SAMPLE_PEAK */ }
  impl Meter { pub fn new(channels: u32, rate: u32) -> Self; pub fn push(&mut self, interleaved: &[f32]); pub fn finish(&self) -> Option<Measured>; }
  ```
- Tags are in dB relative to the ReplayGain reference (~-18 LUFS), so tag → linear = `10^(dB/20)`.
- Measured value → `10^((TARGET_LUFS - lufs)/20)`.
- The peak used is the album peak, otherwise the track peak, otherwise the measured one.
- Use `finish` only if the track was measured for at least 30 s.

- [ ] **Step 1: Write failing tests:**
  - `prefers_album_tag_over_track_tag`: album -6 dB and track -3 dB → `0.501` (±0.001);
  - `falls_back_to_track_tag_then_measured_then_unity`: tags only → measured; measured `(-12.0, 0.5)` → `10^(-6/20)`; nothing → `1.0`;
  - `gain_is_limited_by_peak`: tag +6 dB, peak 0.9 → gain == `1/0.9`;
  - `disabled_is_unity`: `enabled=false` → `1.0`;
  - `meter_measures_sine_near_expected_lufs`: a 1 kHz sine at amplitude 0.5, stereo, 48 kHz, 35 s → lufs ≈ -9.0 ± 0.5;
  - `cache_roundtrip_and_album_average`: two measured tracks, -10 and -14 → `album_lufs == -12`; save/load in `std::env::temp_dir()` keeps the values.
- [ ] **Step 2: Run** `cargo test --lib music::gain` → **Expected:** FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → **Expected:** PASS.
- [ ] **Step 5: Commit** `feat(musica): ganho de álbum (ReplayGain/EBU R128) sem clipping`

---

### Task 5: Fila (Rust puro)

**Files:**
- Create: `src-tauri/src/music/queue.rs`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Copy, Serialize, Deserialize, PartialEq)] #[serde(rename_all = "lowercase")] pub enum Repeat { Off, All, One }
  #[derive(Clone, Serialize, Deserialize)] pub struct QueueTrack { pub path: String, pub size: u64, pub ext: String }
  pub struct Queue { tracks: Vec<QueueTrack>, order: Vec<usize>, pos: usize, pub repeat: Repeat, shuffle: bool }
  impl Queue {
      pub fn new(tracks: Vec<QueueTrack>, start: usize) -> Self;
      pub fn current(&self) -> Option<(usize, &QueueTrack)>;   // índice no álbum
      pub fn peek_next(&self) -> Option<(usize, &QueueTrack)>; // para gapless (respeita repeat)
      pub fn advance(&mut self) -> Option<(usize, &QueueTrack)>; // fim natural da faixa
      pub fn next(&mut self) -> Option<(usize, &QueueTrack)>;    // botão ⏭ (Repeat::One não prende)
      pub fn prev(&mut self) -> Option<(usize, &QueueTrack)>;
      pub fn jump(&mut self, album_index: usize);
      pub fn set_shuffle(&mut self, on: bool, seed: u64); // mantém a faixa atual como primeira
  }
  ```

- [ ] **Step 1: Write failing tests:**
  - `advance_stops_at_end_when_repeat_off`;
  - `repeat_all_wraps`;
  - `repeat_one_repeats_on_advance_but_next_moves`;
  - `prev_at_start_stays`;
  - `shuffle_keeps_current_first_and_is_permutation`: same seed → same order; `order` contains every index once;
  - `unshuffle_resumes_album_order_after_current`;
  - `peek_next_matches_advance`.
- [ ] **Step 2: Run** `cargo test --lib music::queue` → **Expected:** FAIL.
- [ ] **Step 3: Implement.** Shuffle uses a small xorshift seeded by `seed`, with no `rand` dependency.
- [ ] **Step 4: Run** → **Expected:** PASS.
- [ ] **Step 5: Commit** `feat(musica): fila com aleatório e repetir`

---

### Task 6: Motor e saída de áudio (Rust)

**Files:**
- Modify: `Cargo.toml` (add `cpal = "0.15"`)
- Create: `src-tauri/src/music/engine.rs`
- Create: `src-tauri/src/music/output.rs`

**Interfaces:**
- Consumes: `RangeSource`/`CloudFetcher` (T2), `Decoder`/`Resampler` (T3), `track_gain`/`Meter`/`LoudnessCache` (T4), `Queue` (T5).
- Produces:
  ```rust
  pub enum Command { Play { album: String, tracks: Vec<QueueTrack>, index: usize, at: f64 }, Pause, Resume, Toggle, Seek(f64), Next, Prev, Jump(usize), Volume(f32), Shuffle(bool), Repeat(Repeat), Normalize(bool), Stop }
  #[derive(Serialize, Clone)] pub struct PlayerState { pub album: Option<String>, pub index: Option<usize>, pub path: Option<String>, pub playing: bool, pub position: f64, pub duration: Option<f64>, pub buffered: f64 /*0..1*/, pub format: Option<FormatInfo>, pub volume: f32, pub shuffle: bool, pub repeat: Repeat, pub normalize: bool, pub error: Option<String> }
  #[derive(Serialize, Clone)] pub struct FormatInfo { pub codec: String, pub bits: Option<u32>, pub sample_rate: u32, pub lossless: bool, pub resampled_to: Option<u32> }
  pub struct Engine { tx: std::sync::mpsc::Sender<Command> }
  impl Engine { pub fn spawn(app: AppHandle) -> Self; pub fn send(&self, c: Command); }
  // output.rs
  pub struct Output { /* cpal stream + ring buffer (Mutex<VecDeque<f32>> ou ringbuf) */ }
  impl Output { pub fn open(preferred_rate: u32, channels: u16) -> Result<Self, String>; pub fn rate(&self) -> u32; pub fn push(&self, samples: &[f32]) /* bloqueia se cheio (~0,5 s) */; pub fn queued_secs(&self) -> f64; pub fn clear(&self); pub fn pause(&self); pub fn play(&self); }
  ```
- Rules:
  - **Output:** opened with the track's rate when the device supports it (`supported_output_configs`); otherwise the default rate, with a `Resampler`. `FormatInfo.resampled_to` = the device rate when resampling happens.
  - **Gapless:**
    - when `queued_secs` + remaining samples < 5 s, the decoder thread opens `peek_next` (fetch + `Decoder::open`) and pushes its samples right after the current track's last one;
    - if the next track's rate ≠ the output's, resample to the output's rate (do not recreate the stream);
    - when the next track starts, `index`/`position` switch to it.
  - **Gain:** `samples × volume × track_gain`, in `f32`. Volume uses a perceptual curve `v²`, where `v` is the bar's 0..1 value.
  - **Meter:** fed with what was decoded, before the gain. At the end of the track (or after ≥30 s when changing tracks), save it in `LoudnessCache` at `app_data_dir()/music-cache.json`.
  - **`music://state`** is emitted every 250 ms while playing, and right away on every command.
  - **Errors:** a track that fails to open shows `error` and advances to the next one after 2 s. A decoder error mid-track is treated as the end of the track.
- No unit test for `Output`, which needs a device. `Engine` gets one integration test with a `MemFetcher` and a `NullOutput` (a trait `Sink` between engine and output: `trait Sink { fn rate(&self)->u32; fn push(&self,&[f32]); fn queued_secs(&self)->f64; fn clear(&self); fn pause(&self); fn play(&self); }`).

- [ ] **Step 1: Write failing tests** in `engine.rs`, with `NullSink { rate: 48000, pushed: Mutex<usize> }` and two 1 s WAVs, one at 44100 and one at 48000:
  - `plays_album_gapless_across_rates`: after `Play` index 0 with repeat Off, the total pushed ≈ 2 s × 48000 × 2 channels (±2 %), with no gap between tracks (count the samples);
  - `next_and_prev_change_index`;
  - `stop_clears`.
  - `Engine` takes the `Fetcher` factory and the `Sink` as dependencies: `Engine::spawn_with(fetch: Arc<dyn Fn(&QueueTrack)->Arc<dyn Fetcher>+Send+Sync>, sink: Arc<dyn Sink>, on_state: Box<dyn Fn(PlayerState)+Send>, cache_path: Option<PathBuf>)`.
  - `Engine::spawn(app)` builds the real ones.
- [ ] **Step 2: Run** `cargo test --lib music::engine` → **Expected:** FAIL.
- [ ] **Step 3: Implement** `engine.rs` and `output.rs` (`CpalSink`).
- [ ] **Step 4: Run** → **Expected:** PASS; clippy clean.
- [ ] **Step 5: Commit** `feat(musica): motor de reprodução gapless com saída cpal`

---

### Task 7: Comandos, metadados e teclas de mídia (Rust)

**Files:**
- Modify: `Cargo.toml` (add `souvlaki = "0.8"`)
- Create: `src-tauri/src/music/commands.rs`
- Create: `src-tauri/src/music/media_keys.rs`
- Modify: `src-tauri/src/music/mod.rs` (`setup(app)`: `Engine::spawn`, `app.manage`, start the media keys)
- Modify: `src-tauri/src/lib.rs` (`music::setup(app)?` and register the commands)

**Interfaces:**
- Produces (Tauri commands, used by the TS in Task 8):
  ```rust
  #[tauri::command] music_play(state, album: String, tracks: Vec<QueueTrack>, index: usize, at: f64)
  #[tauri::command] music_command(state, cmd: String /* "pause"|"resume"|"toggle"|"next"|"prev"|"stop" */)
  #[tauri::command] music_seek(state, secs: f64)
  #[tauri::command] music_jump(state, index: usize)
  #[tauri::command] music_set(state, volume: Option<f32>, shuffle: Option<bool>, repeat: Option<Repeat>, normalize: Option<bool>)
  #[tauri::command] async music_meta(app, path: String, size: u64, ext: String) -> Result<TrackMeta, String>
  #[derive(Serialize)] pub struct TrackMeta { pub title: Option<String>, pub artist: Option<String>, pub duration: Option<f64>, pub codec: String, pub bits: Option<u32>, pub sample_rate: u32, pub lossless: bool, pub cover: Option<String> /* data URL, só se pedido */ }
  ```
- `music_meta`:
  - reads `0..=262143` with `reader::fetch_range` (make it `pub(crate)`);
  - calls `probe_prefix`;
  - stores the result in `music-cache.json` (`meta` section, key `path|size`, without the cover);
  - on a 429, waits `Retry-After`/5 s and tries again (up to 5 times).
  - The cover only comes as a data URL when the TS sends `want_cover: bool`; add the parameter.
- `media_keys.rs`:
  - `souvlaki::MediaControls` with the main window's `hwnd` (`window.hwnd()` on Windows);
  - play/pause/next/prev events → `Engine::send`;
  - updates the metadata (title, album) and the state on every track change.
  - On other systems, `cfg` out the HWND and do not fail.

- [ ] **Step 1: Write failing test** `commands::tests::parses_command_names`: `parse_cmd("toggle") == Some(Command::Toggle)` … `parse_cmd("x") == None`.
- [ ] **Step 2: Run** `cargo test --lib music::commands` → **Expected:** FAIL.
- [ ] **Step 3: Implement** the commands, `setup` and the media keys.
- [ ] **Step 4: Run** `cargo test` (all) + `cargo clippy --all-targets` → **Expected:** PASS, 0 warnings.
- [ ] **Step 5: Commit** `feat(musica): comandos, metadados e teclas de mídia`

---

### Task 8: Estado e preferências no front (TS)

**Files:**
- Create: `src/lib/music/api.ts`
- Create: `src/lib/music/session.ts`
- Create: `src/lib/music/store.ts`
- Test: `src/lib/music/store.test.ts`

**Interfaces:**
- Consumes: Task 7 commands, `Album`/`Track` (Task 1).
- Produces:
  ```ts
  // api.ts
  export interface FormatInfo { codec: string; bits: number | null; sample_rate: number; lossless: boolean; resampled_to: number | null }
  export interface PlayerState { album: string | null; index: number | null; path: string | null; playing: boolean; position: number; duration: number | null; buffered: number; format: FormatInfo | null; volume: number; shuffle: boolean; repeat: "off" | "all" | "one"; normalize: boolean; error: string | null }
  export interface TrackMeta { title: string | null; artist: string | null; duration: number | null; codec: string; bits: number | null; sample_rate: number; lossless: boolean; cover: string | null }
  export const musicApi: { play(album: Album, index: number, at?: number): Promise<void>; command(c: "pause"|"resume"|"toggle"|"next"|"prev"|"stop"): Promise<void>; seek(s: number): Promise<void>; jump(i: number): Promise<void>; set(o: Partial<{ volume: number; shuffle: boolean; repeat: PlayerState["repeat"]; normalize: boolean }>): Promise<void>; meta(t: Track, wantCover?: boolean): Promise<TrackMeta>; onState(cb: (s: PlayerState) => void): Promise<UnlistenFn> };
  // store.ts — localStorage "higanbana:music:v1"
  export interface AlbumProgress { index: number; position: number; updatedAt: number }
  export interface MusicPrefs { volume: number /*0..1, padrão 0.8*/; shuffle: boolean; repeat: "off"|"all"|"one" /*padrão "all"*/; normalize: boolean /*padrão true*/ }
  export const musicStore: { progress(albumPath: string): AlbumProgress | undefined; setProgress(albumPath: string, index: number, position: number): void; prefs(): MusicPrefs; setPrefs(p: Partial<MusicPrefs>): void };
  // session.ts — estado global reativo
  export const music: { state: Accessor<PlayerState | null>; album: Accessor<Album | null>; meta: (path: string) => TrackMeta | undefined; open(album: Album, index: number, at?: number): void; loadMeta(album: Album): void /* fila sequencial, 1 por vez, cancela ao trocar de álbum */ };
  ```
- `session.ts`:
  - escuta `onState` uma vez;
  - salva o progresso a cada 5 s tocando e ao pausar ou trocar de faixa;
  - aplica as `prefs` no início (`set`);
  - pausa a música quando `player.current()` vira não-nulo (o player de vídeo).

- [ ] **Step 1: Write failing tests** (com `localStorage` falso, como em `src/lib/reader` se houver; senão um `Map`):
  - `prefs default and merge`: `prefs()` sem nada salvo é `{ volume: 0.8, shuffle: false, repeat: "all", normalize: true }`, e `setPrefs({volume:.5})` mantém o resto;
  - `progress roundtrip`;
  - `corrupt storage falls back to defaults`.
- [ ] **Step 2: Run** `bun run test src/lib/music` → **Expected:** FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → **Expected:** PASS; `bunx tsc --noEmit` limpo.
- [ ] **Step 5: Commit** `feat(musica): estado, comandos e preferências no front`

---

### Task 9: Encarte e barra (UI)

**Files:**
- Create: `src/views/AlbumView.tsx`
- Create: `src/components/music/MusicBar.tsx`
- Create: `src/components/music/Thread.tsx` (o fio, como barra de busca; se der para reaproveitar o `Thread` de `ReaderView.tsx`, extrair para `src/components/Thread.tsx` e usar nos dois)
- Modify: `src/App.tsx` (a `MusicBar` entre `<div class="flex min-h-0 flex-1">` e o `Toaster`, visível quando `music.state()?.album`)
- Modify: `src/views/FolderView.tsx` (se `buildAlbum(...)` ≠ null → `<AlbumView>`; listar subpastas de disco com `isDiscDir`, uma por vez, antes de montar)
- Modify: `src/views/TitleView.tsx` (seção "Músicas" quando a pasta aberta tem faixas: lista compacta com play e o link "Abrir encarte" → `router.go(filesPath())` mostrando o `AlbumView`; usar o mesmo critério do `FolderView`)

**Interfaces:**
- Consumes: `buildAlbum`, `formatDuration` (T1), `music`, `musicStore` (T8).
- Layout B (o mockup aprovado está em `.superpowers/brainstorm/1445-1793599640/content/player-layout.html`, opção B):
  - **Coluna esquerda** (`w-72`, `sticky top-6`):
    - capa `aspect-square` com `rounded-md`;
    - o **obi** fica sobre a borda direita da capa: `w-9`, `bg-brand`, `writing-mode: vertical-rl`, `font-mincho`, `tracking-[0.2em]`, texto branco, com `catalog ?? year`;
    - título em `font-mincho text-2xl`, linha "obra · ano", e os botões "Tocar" (primário) e "Continuar faixa N (m:ss)" quando houver progresso;
    - linha de formato da faixa atual, ou da primeira com metadados: `FLAC · 16 bit / 44,1 kHz · sem perdas`. Com `resampled_to`, acrescentar `→ 48 kHz`.
  - **Coluna direita:**
    - abas "Faixas" | "Arquivos (N)", onde Arquivos usa o `FileList` existente com `others` e as subpastas;
    - a lista mostra o separador "Disco N" quando há mais de um disco;
    - cada linha tem o número `font-mincho text-xl tabular-nums` (`01`), o título (das tags quando chegam), o artista em `text-muted` se diferir do álbum, e a duração;
    - a faixa atual fica com `text-brand-ink` e `bg-brand/10`, e um ícone de onda no lugar do número quando está tocando;
    - clique na linha → `music.open(album, i)`.
  - Retícula `screentone` de fundo na coluna esquerda, como nas margens do leitor.
  - Capa: `pickCover` → URL. Usar a mesma rota de imagem que o app já usa para arquivos da nuvem; se não houver, um `music_meta(want_cover)` da primeira faixa. Sem nada, um quadro com o `catalog` em mincho.
- **`MusicBar`** (`h-16`, `border-t border-line`, `bg-surface`):
  - play/pausa;
  - "título — obra" (clique leva ao álbum);
  - o **fio** como barra de busca: linha `1px` `bg-line-strong`, trecho tocado `2px` `bg-brand`, trecho em buffer `bg-fg/20`, um traço vertical `bg-brand-ink` na posição; clique ou arraste faz seek;
  - tempo `m:ss / m:ss`;
  - anterior, próxima, aleatório (`Shuffle`), repetir (`Repeat`/`Repeat1`);
  - volume (slider; `M` muda);
  - um botão "volume igualado" (`AudioLines`), ativo/inativo, com o título explicando.
- **Atalhos com o foco na página do álbum ou na barra:** espaço = toggle, ←/→ = ±5 s. Ignorar quando o foco está em input.
- Estados:
  - carregando a primeira faixa: spinner discreto no lugar do play;
  - `error`: um toast curto com a mensagem.

- [ ] **Step 1: Write failing test** para a única lógica nova fora da UI: `albumFormatLabel(info)` em `src/lib/music/logic.ts`:
  - `{codec:"FLAC",bits:16,sample_rate:44100,lossless:true}` → `"FLAC · 16 bit / 44,1 kHz · sem perdas"`;
  - `{codec:"MP3",bits:null,sample_rate:44100,lossless:false}` → `"MP3 · 44,1 kHz"`;
  - com `resampled_to:48000` → sufixo `" → 48 kHz"`.
- [ ] **Step 2: Run** `bun run test src/lib/music` → **Expected:** FAIL.
- [ ] **Step 3: Implement** `albumFormatLabel`, `AlbumView`, `MusicBar` e as integrações.
- [ ] **Step 4: Run** `bun run test` + `bunx tsc --noEmit` → **Expected:** PASS.
- [ ] **Step 5: Commit** `feat(musica): encarte do álbum e barra do player`

---

### Task 10: Verificação no app (E2E manual)

**Files:** nenhum permanente. As sondas temporárias seguem o padrão já usado: `src/probe.ts` + um comando `debug_dump`, removidos ao fim.

- [ ] **Step 1:** subir o app (`bun tauri dev`, ou vite + `cargo build` com `TAURI_CONFIG` se o do dono estiver aberto), e capturar com `PrintWindow` só a janela do app.
- [ ] **Step 2:** abrir `OSTs/Animes/A/<A-Channel…>/2011.05.25 - A-Channel TV1 OP1 - Morning Arch (SVWC-7767)`. Esperado:
  - encarte com o obi `SVWC-7767`;
  - 4 faixas cujas durações aparecem uma a uma;
  - formato `FLAC · 16 bit / 44,1 kHz · sem perdas`.
- [ ] **Step 3:** Tocar. Esperado:
  - som em menos de 2 s;
  - a barra avança;
  - navegar para outra pasta não interrompe a música;
  - um seek para 80 % antes do download terminar toca dali;
  - o fim da faixa 1 emenda na 2 sem silêncio extra.
- [ ] **Step 4:** abrir um álbum MP3 (por exemplo `OSTs/Animes/0-9/-Hack/...`) e um com WAV. Esperado:
  - o formato aparece certo;
  - num WAV de 96 kHz, se houver, aparece `→ 48 kHz`.
- [ ] **Step 5:** abrir uma pasta só com APE+CUE (Ghibli, `[APE]`). Esperado: a lista comum de arquivos, sem encarte.
- [ ] **Step 6:** abrir um vídeo com música tocando → a música pausa. Abrir o leitor → a música continua.
- [ ] **Step 7:** conferir que nenhum arquivo de áudio foi criado (`app_data_dir` só tem o `music-cache.json`, e o `%TEMP%` não tem nada novo do app).
- [ ] **Step 8:** remover as sondas; rodar `cargo test`, `cargo clippy --all-targets`, `bun run test` e `bunx tsc --noEmit`, todos limpos.

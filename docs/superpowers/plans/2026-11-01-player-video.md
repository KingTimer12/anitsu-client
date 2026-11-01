# Player de vídeo — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** assistir episódios de anime/dorama direto da nuvem, em streaming por pedaços e sem gravar nada em disco. O player usa o mpv como motor e uma interface própria no layout "Imersivo", com pular abertura, próximo episódio automático, continuar de onde parou e faixas lembradas.

**Architecture:**
- O mpv (libmpv, via `tauri-plugin-libmpv`) desenha atrás de um webview transparente, e a interface SolidJS fica por cima.
- O mpv lê o vídeo de um proxy HTTP local no Rust (`127.0.0.1`, porta aleatória, token na URL). O proxy repassa os `Range` para `/api/preview` da nuvem com o cookie de sessão, que só existe no Rust.
- Tempos de abertura e encerramento: primeiro os capítulos do arquivo (lidos no TS); se não houver, o AniSkip (Rust, com cache em disco).
- Progresso, faixas e configurações ficam em `player.json` no `app_data_dir`.

**Tech Stack:**
- Rust: Tauri 2, tokio, hyper 1 (servidor do proxy), reqwest via `tauri-plugin-http`.
- Frontend: SolidJS, Tailwind v4, lucide-solid, `tauri-plugin-libmpv` + `tauri-plugin-libmpv-api`, vitest (novo, só para lógica pura em TS).

**Spec:** `docs/superpowers/specs/2026-11-01-player-video-design.md`

## Global Constraints

- **Nada do vídeo vai para o disco.** mpv com `cache=yes`, `cache-on-disk=no`, `demuxer-max-bytes` padrão `150MiB` (configurável) e `demuxer-max-back-bytes=50MiB`.
- **Tokens só no Rust.** O cookie nunca vai para o JS nem para opções ou argumentos do mpv. O mpv só conhece `http://127.0.0.1:<porta>/<token>/stream?...`.
- **Proxy:**
  - escuta só em `127.0.0.1`;
  - token aleatório de 32 bytes em hex (usar o `getrandom` que já existe);
  - requisição sem o token → `403`;
  - **no máximo 2 conexões simultâneas com a nuvem por arquivo**;
  - conexão do mpv fechada → requisição de cima cancelada.
- **Endpoint de cima:** `GET {nuvem}/api/preview?path=<caminho>`, via `AnitsuHttp` da nuvem (`apis.nuvem.send(...)`), que já renova em 401.
- **AniSkip:**
  - endpoint `GET https://api.aniskip.com/v2/skip-times/{malId}/{ep}?types[]=op&types[]=ed&episodeLength={segundos inteiros}`;
  - só quando o arquivo não tem capítulos de abertura/encerramento, a obra é da seção `Animes` (não `Doramas`), há `idMal` e há número de episódio;
  - cache em disco por `"{malId}:{ep}"`, inclusive para "não encontrado", com TTL de 30 dias.
- **Visual:**
  - layout A "Imersivo", com cores e fontes de `src/styles/app.css` (`--color-bg`, `--accent`, `--accent-ink`, `--color-line-strong`, `font-display`/Outfit, Inter, `--ease-expo`);
  - ícones `lucide-solid`;
  - nenhuma OSC/OSD do mpv: `osc=no`, `osd-level=0`, `input-default-bindings=no`, `input-vo-keyboard=no`.
- **Textos da interface** (pt-BR, exatos):
  - `Pular abertura`
  - `Próximo episódio em {n}`
  - `Assistir agora`
  - `Cancelar`
  - `Assistir`
  - `Continuar ep. {n} ({mm:ss})`
  - `Legendas`, `Áudio`, `Desligada`
  - `Ir para o próximo episódio sozinho`
  - `Tempo da contagem`
  - `Mostrar "Pular abertura"`
  - `Memória à frente`
  - `Sem conexão com a nuvem. Tentando de novo…`
- **Configurações padrão:**
  - `autoNext: true`
  - `countdownSec: 5` (opções 5, 10 ou 15)
  - `showSkipIntro: true`
  - `subLang: "por"`
  - `audioLang: "jpn"`
  - `cacheMiB: 150` (opções 100, 150, 300 ou 500)
- **"Visto":** posição ≥ 90% da duração ou posição ≥ início do encerramento.
- **Atalhos:**

  | Tecla | Ação |
  |---|---|
  | Espaço | pausa |
  | ← / → | ±5 s |
  | ↑ / ↓ | volume ±5 |
  | M | mudo |
  | F e duplo clique | tela cheia |
  | S | pular abertura |
  | N | próximo episódio |
  | Esc | sai da tela cheia, depois fecha o player |

- **Controles somem** após `2500` ms sem mexer o mouse, só com o vídeo tocando.
- **Extensões de vídeo:** `.mkv .mp4 .avi .webm .m4v .mov .ts .wmv .flv`.
- **Commits:**
  - mensagens em português, estilo do repo (`feat: …`, `chore: …`);
  - **nunca** incluir `Co-Authored-By` nem outra atribuição a IA;
  - commitar só se o dono autorizou na execução.
- **Plano B (decisão do dono):** se a Task 1 reprovar o embutido na janela principal, o player vira uma janela Tauri própria, label `player`, sem bordas e com a mesma interface. As demais tasks não mudam, exceto a Task 8, que tem as duas variantes descritas.

## Review Focus

1. **Nomes de arquivo com caracteres especiais** (`[`, `]`, `&`, `#`, `'`, acentos, espaços, `_`) no caminho → o proxy precisa chegar exatamente ao mesmo caminho na nuvem. Testado na Task 2 (`url_roundtrip_special_chars`).
2. **Arrastar a barra de progresso** (rajada de seeks) → nunca mais de 2 conexões com a nuvem, e as abandonadas são canceladas. Testado na Task 2 (`caps_upstream_concurrency`, `cancels_on_client_drop`).
3. **Número de episódio com `_01_`, ` - 01 `, `E01`, `S01E01`, `Ep. 01`** (todos existem no acervo; o `parseEpisode` atual não pega `_01_` nem `S01E01`) → o AniSkip não pode ser consultado com número errado ou nulo quando dá para extrair. Testado na Task 4 (`episodeNumber`).
4. **Capítulos com títulos variados** ("Opening", "OP", "Intro", "Avant", "Part A", "Ending", "ED", "Preview" depois do ED, sem capítulos) → abertura e encerramento corretos. O fim do encerramento é o início do capítulo seguinte. "Preview" depois do ED não pode virar encerramento. Testado na Task 4 (`classifyChapters`).
5. **Pasta com arquivos que não são vídeo** (`.ass`, `.txt`, `.nfo`, subpastas) **e o último episódio** → o próximo episódio ignora o que não é vídeo e é `null` no último. Testado na Task 4 (`nextEpisode`).

---

## Arquivos

**Rust, `src-tauri/src/player/`:**

| Arquivo | Conteúdo |
|---|---|
| `mod.rs` | `setup`, state `PlayerState` e reexports |
| `proxy.rs` | servidor HTTP local, trait `Upstream`, `NuvemUpstream` |
| `aniskip.rs` | cliente e parse do AniSkip, cache |
| `store.rs` | `player.json` (progresso, faixas, configurações) |
| `commands.rs` | comandos Tauri |

**Frontend:**

| Arquivo | Conteúdo |
|---|---|
| `src/lib/player/logic.ts` (+ `logic.test.ts`) | lógica pura: capítulos, próximo episódio, número de episódio, visto, faixas, contagem |
| `src/lib/player/mpv.ts` | wrapper fino do `tauri-plugin-libmpv-api` |
| `src/lib/player/api.ts` | `invoke` dos comandos `player_*` |
| `src/lib/player/store.ts` | store reativa (progresso, configurações, faixas), carregada uma vez |
| `src/lib/player/session.ts` | o que está tocando (`openPlayer`, `closePlayer`) |
| `src/views/PlayerView.tsx` | tela do player (layout A) |
| `src/components/player/SeekBar.tsx` | barra de progresso |
| `src/components/player/Controls.tsx` | linha de controles |
| `src/components/player/SkipButton.tsx` | botão "Pular abertura" |
| `src/components/player/NextCard.tsx` | cartão do próximo episódio |
| `src/components/player/TracksMenu.tsx` | menu de legendas e áudio |
| `src/components/player/SettingsMenu.tsx` | menu de configurações |
| `src/components/player/EpisodesPanel.tsx` | painel de episódios |

**Modificados:**
- `src-tauri/Cargo.toml`
- `src-tauri/src/lib.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/default.json`
- `package.json`
- `vite.config.ts`
- `src/styles/app.css`
- `src/App.tsx`
- `src/components/FileList.tsx`
- `src/views/TitleView.tsx`
- `.gitignore`
- `README.md`

---

### Task 1: Spike — mpv embutido atrás do webview

Objetivo: decidir entre "mesma janela" e o Plano B, e deixar o plugin instalado. Este é o único passo sem TDD. Ele termina com uma decisão registrada.

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `package.json`, `.gitignore`
- Create: `src/lib/player/mpv.ts`
- Create (temporário, apagar ao fim): `src/__spike__/MpvSpike.tsx`

**Interfaces:**
- Produces, em `src/lib/player/mpv.ts`:
  ```ts
  export interface MpvEvents { onProp(name: string, value: unknown): void; onEnd(reason: string): void }
  export interface Mpv {
    load(url: string, startSec?: number): Promise<void>;
    set(name: string, value: string | number | boolean): Promise<void>;
    get<T>(name: string): Promise<T>;
    command(...args: (string | number)[]): Promise<void>;
    destroy(): Promise<void>;
  }
  export function createMpv(options: Record<string, string>, observe: string[], events: MpvEvents): Promise<Mpv>;
  ```
  Observar no mínimo `time-pos`, `duration`, `pause`, `volume`, `mute`, `paused-for-cache`, `demuxer-cache-time`, `chapter-list`, `track-list`, `eof-reached`.

- [ ] **Step 1: Instalar o plugin.**
  - Rodar `cargo add tauri-plugin-libmpv` em `src-tauri` e `bun add tauri-plugin-libmpv-api`.
  - Seguir o README do plugin (versão 0.3.x) para baixar a libmpv e o wrapper no Windows.
  - Colocar as DLLs no local que o README indica e adicionar esse diretório ao `.gitignore`.
  - Registrar `.plugin(tauri_plugin_libmpv::init())` em `lib.rs` e as permissões do plugin na capability.
- [ ] **Step 2: Janela transparente.**
  - Em `tauri.conf.json` (janela `main`), usar `"transparent": true` e remover `backgroundColor`.
  - Em `app.css`, `html, body` passam a ter `background: transparent`.
  - O fundo opaco passa para o contêiner raiz do `App.tsx` (ele já tem `bg-bg`) e para o fallback de `ready`.
  - Conferir que login, home e página da obra continuam opacos.
- [ ] **Step 3: Implementar `createMpv` em `mpv.ts`** sobre a API do plugin. As opções fixas da seção Global Constraints são passadas aqui: `osc`, `osd-level`, cache e `input-*`.
- [ ] **Step 4: Spike.** `MpvSpike.tsx` monta o `createMpv`, toca `https://test-videos.co.uk/vids/bigbuckbunny/mp4/h264/1080/Big_Buck_Bunny_1080_10s_1MB.mp4` (ou um arquivo local) e põe uma barra HTML semitransparente por cima. Montar temporariamente no lugar do `HomeView` e rodar `bun tauri dev`.
- [ ] **Step 5: Avaliar com estes critérios** (todos precisam passar):
  1. Vídeo visível atrás da interface HTML, e a interface clicável.
  2. Redimensionar e maximizar pela barra de título própria sem o vídeo descolar e sem piscar por mais de um frame.
  3. Tela cheia (`getCurrentWindow().setFullscreen(true)`) e volta sem tela preta.
  4. Minimizar e restaurar continua tocando.
  5. Sombra e cantos da janela iguais aos de antes da transparência.

  Se algum falhar, aplicar o **Plano B**:
  - reverter a transparência da janela `main`;
  - criar a janela `player` sob demanda (Task 8, variante B), com `transparent: true` e `decorations: false`;
  - repetir os critérios nela.
- [ ] **Step 6: Registrar o resultado** em uma linha na spec, seção "Embutir o mpv na janela": `Resultado do spike (data): mesma janela | Plano B — motivo`. Apagar `src/__spike__`.
- [ ] **Step 7: Verificar o build.** `bunx tsc --noEmit` e `cargo clippy` sem erros.
- [ ] **Step 8: Commit.** `chore: embute o mpv (tauri-plugin-libmpv) e decide o modo da janela`

---

### Task 2: Proxy de stream local

**Files:**
- Create: `src-tauri/src/player/mod.rs`, `src-tauri/src/player/proxy.rs`, `src-tauri/src/player/commands.rs`
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs` (`mod player;` e `player::setup(app)?` depois de `http::setup`)

**Interfaces:**
- Produces, em `proxy.rs`:
  ```rust
  pub struct UpstreamResponse {
      pub status: u16,
      pub headers: Vec<(String, String)>, // só Content-Type, Content-Length, Content-Range, Accept-Ranges
      pub body: Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>,
  }
  pub trait Upstream: Send + Sync + 'static { // sem async_trait: devolve BoxFuture de futures_util
      fn fetch(&self, path: &str, range: Option<&str>) -> BoxFuture<'static, Result<UpstreamResponse, String>>;
  }
  pub struct StreamProxy { /* port, token */ }
  impl StreamProxy {
      pub async fn start<U: Upstream>(upstream: U) -> std::io::Result<StreamProxy>;
      pub fn url_for(&self, nuvem_path: &str) -> String; // http://127.0.0.1:{port}/{token}/stream?path={urlencoded}
  }
  pub struct NuvemUpstream { app: tauri::AppHandle } // usa app.state::<AnitsuApis>().nuvem.send(&auth, GET, "preview", |r| r.query(&[("path", path)]).header(RANGE, range))
  ```
- Produces, em `mod.rs`: `pub struct PlayerState { pub proxy: StreamProxy, ... }`, gerenciado com `app.manage`.
- Dependências novas: `hyper = { version = "1", features = ["server", "http1"] }`, `hyper-util = { version = "0.1", features = ["tokio"] }`, `http-body-util = "0.1"`, `bytes = "1"`, `futures-util = "0.3"`, `urlencoding = "2"`.

- [ ] **Step 1: Escrever os testes que falham** em `proxy.rs` (`#[cfg(test)]`), com um `FakeUpstream` que serve um `Vec<u8>` de 1 MiB respeitando `Range` e conta as conexões ativas (um `AtomicUsize` incrementado no `fetch` e decrementado quando o stream do corpo é dropado). Usar `reqwest` (do `tauri_plugin_http::reqwest`) como cliente.
  - `serves_range`: `GET url_for("a/b.mkv")` com `Range: bytes=100-199` → status `206`, corpo igual a `data[100..200]`, header `Content-Range: bytes 100-199/1048576`.
  - `rejects_without_token`: `GET http://127.0.0.1:{port}/errado/stream?path=x` → `403`, e o upstream não é chamado.
  - `url_roundtrip_special_chars`: para `"Animes/Letra F/Fairy Gone/[FênixFansub_&_IsekaiSubs]_Fairy_Gone_01_[HDTV] #1 'x'.mp4"`, o `path` que o `FakeUpstream` recebe é idêntico.
  - `caps_upstream_concurrency`: 5 requisições simultâneas ao mesmo `path`, com o fake demorando 200 ms para começar → o pico de conexões ativas no fake é ≤ 2, e as 5 terminam com `206`.
  - `cancels_on_client_drop`: começar um GET de 1 MiB com o fake liberando um pedaço a cada 50 ms, ler 1 pedaço e dropar a resposta → em até 500 ms o contador de ativas volta a 0.
- [ ] **Step 2: Rodar e ver falhar.** `cargo test player::proxy` → falha de compilação: `StreamProxy` não existe.
- [ ] **Step 3: Implementar `StreamProxy::start`.**
  - `TcpListener::bind("127.0.0.1:0")`, `hyper::server::conn::http1` por conexão, em `tokio::spawn`.
  - Token: 32 bytes do `getrandom` em hex.
  - Limite por `path`: `HashMap<String, Arc<Semaphore>>` com 2 permissões. A permissão fica junto do stream do corpo e é solta no drop, o que também cancela a requisição de cima.
  - Repassar o status de cima e só os 4 headers listados.
  - Erro de cima → `502`.
- [ ] **Step 4: Rodar e ver passar.** `cargo test player::proxy` → 5 passed.
- [ ] **Step 5: Implementar `NuvemUpstream` e `player::setup`.**
  - `setup` sobe o proxy no runtime do Tauri (`tauri::async_runtime::block_on` dentro de `setup`) e faz `app.manage(PlayerState { proxy, .. })`.
  - Comando `player_stream_url(path: String, size: u64) -> String` em `commands.rs`: se `download_dir()/sanitize_file_name(nome)` existe com tamanho igual a `size`, devolve o caminho local absoluto; senão devolve `proxy.url_for(&path)`.
  - Registrar em `generate_handler!`.
- [ ] **Step 6: Verificar.** `cargo clippy` e `cargo test` sem erros.
- [ ] **Step 7: Commit.** `feat: proxy local de stream da nuvem para o player`

---

### Task 3: AniSkip e `player.json` (Rust)

**Files:**
- Create: `src-tauri/src/player/aniskip.rs`, `src-tauri/src/player/store.rs`
- Modify: `src-tauri/src/player/commands.rs`
- Modify: `src-tauri/src/player/mod.rs`, `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `crate::anilist::cache::DiskCache<V>` (`open(path, ttl_secs)`, `get`, `insert`, `flush`). Se o módulo `cache` for privado, torná-lo `pub(crate)`.
- Produces, em `aniskip.rs`:
  ```rust
  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)] pub struct Interval { pub start: f64, pub end: f64 }
  #[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)] pub struct SkipTimes { pub op: Option<Interval>, pub ed: Option<Interval> }
  pub fn parse_response(status: u16, body: &str) -> Option<SkipTimes>; // None = erro transitório (não cachear)
  ```
- Produces, em `store.rs` (serde `camelCase`):
  ```rust
  pub struct Progress { pub position: f64, pub duration: f64, pub watched: bool, pub updated_at: i64 }
  pub struct TrackPref { pub sub_lang: Option<String>, pub sub_title: Option<String>, pub sub_off: bool, pub audio_lang: Option<String>, pub audio_title: Option<String> }
  pub struct Settings { pub auto_next: bool, pub countdown_sec: u8, pub show_skip_intro: bool, pub sub_lang: String, pub audio_lang: String, pub cache_mib: u32 } // Default = valores da seção Global Constraints
  #[derive(Default)] pub struct PlayerData { pub progress: HashMap<String, Progress>, pub tracks: HashMap<String, TrackPref>, pub settings: Settings }
  pub struct PlayerStore { /* path + Mutex<PlayerData> */ }
  impl PlayerStore { pub fn open(path: PathBuf) -> Self; pub fn data(&self) -> PlayerData; pub fn update(&self, f: impl FnOnce(&mut PlayerData)); /* grava em disco */ }
  ```
- Produces, comandos:
  - `player_data() -> PlayerData`
  - `player_save_progress(path: String, progress: Progress)`
  - `player_save_tracks(release: String, pref: TrackPref)`
  - `player_save_settings(settings: Settings)`
  - `player_aniskip(mal_id: u32, episode: u32, duration: f64) -> SkipTimes`

- [ ] **Step 1: Escrever os testes que falham.**
  - `aniskip::parse_found`: body `{"found":true,"results":[{"interval":{"startTime":90.5,"endTime":180.0},"skipType":"op","skipId":"x","episodeLength":1420.0},{"interval":{"startTime":1290.0,"endTime":1380.0},"skipType":"ed","skipId":"y","episodeLength":1420.0}],"message":"Successfully found skip times","statusCode":200}` com status 200 → `op = 90.5..180`, `ed = 1290..1380`.
  - `aniskip::parse_not_found`: status 404, body `{"found":false,"results":[],"message":"No skip times found","statusCode":404}` → `Some(SkipTimes::default())`.
  - `aniskip::parse_server_error`: status 500 → `None`.
  - `store::defaults`: `PlayerData::default().settings` igual a `{auto_next: true, countdown_sec: 5, show_skip_intro: true, sub_lang: "por", audio_lang: "jpn", cache_mib: 150}`.
  - `store::roundtrip`: `open(tmp)`, depois `update` inserindo progresso, depois um novo `open(tmp)` → mesmo `data()`.
  - `store::corrupt_file_falls_back`: arquivo com `{lixo` → `data()` igual ao padrão, sem panic.
- [ ] **Step 2: Rodar e ver falhar.** `cargo test player::` → falha de compilação.
- [ ] **Step 3: Implementar `parse_response` e `PlayerStore`.** Gravação atômica: escreve `player.json.tmp` e renomeia.
- [ ] **Step 4: Implementar `player_aniskip`.**
  - Cache `DiskCache<SkipTimes>` em `app_data_dir/aniskip.json`, TTL de `30 * 24 * 3600`.
  - Chave `"{mal_id}:{episode}"`.
  - Requisição com o reqwest do `tauri_plugin_http` (sem cookie; é uma API pública), `episodeLength = duration.round()`.
  - Resultado `None` (erro) não é cacheado e devolve `SkipTimes::default()`.
  - `flush` depois de inserir.
- [ ] **Step 5: Rodar e ver passar.** `cargo test player::` → todos passam. `cargo clippy` sem erros.
- [ ] **Step 6: Commit.** `feat: AniSkip com cache e armazenamento do player`

---

### Task 4: Lógica pura do player (TS + vitest)

**Files:**
- Modify: `package.json` (`bun add -d vitest`; script `"test": "vitest run"`), `vite.config.ts` (`test: { environment: "node", include: ["src/**/*.test.ts"] }`)
- Create: `src/lib/player/logic.ts`, `src/lib/player/logic.test.ts`

**Interfaces:**
- Consumes: `FileEntry` (`src/lib/anitsu/nuvem.ts`), `parseEpisode` (`src/lib/library.ts`).
- Produces:
  ```ts
  export interface Interval { start: number; end: number }
  export interface SkipTimes { op: Interval | null; ed: Interval | null }
  export interface Chapter { title: string; time: number }
  export interface Track { id: number; type: "sub" | "audio" | "video"; lang?: string; title?: string; selected: boolean }
  export interface TrackPref { subLang?: string; subTitle?: string; subOff: boolean; audioLang?: string; audioTitle?: string }
  export const VIDEO_EXT: Set<string>; // valores da seção Global Constraints
  export function classifyChapters(chapters: Chapter[], duration: number): SkipTimes;
  export function episodeNumber(fileName: string): number | null;
  export function videoFiles(files: FileEntry[]): FileEntry[]; // só vídeo, ordem natural pt-BR numeric (igual ao FileList)
  export function nextEpisode(files: FileEntry[], currentName: string): FileEntry | null;
  export function isWatched(position: number, duration: number, ed: Interval | null): boolean;
  export function nextCardStart(duration: number, ed: Interval | null, countdownSec: number): number; // ed?.start ?? duration - countdownSec
  export function pickTracks(tracks: Track[], pref: TrackPref | undefined, defaults: { subLang: string; audioLang: string }): { sid: number | "no" | null; aid: number | null }; // null = deixa como está
  export function formatTime(sec: number): string; // 75 → "1:15", 3725 → "1:02:05"
  ```

- [ ] **Step 1: Escrever os testes que falham** em `logic.test.ts`:
  - `classifyChapters`:
    - `[{Avant,0},{Opening,62},{Part A,152},{Part B,700},{Ending,1290},{Preview,1380}]`, duração 1420 → `op {62,152}`, `ed {1290,1380}`.
    - `[{Intro,0},{Episode,90},{Credits,1300}]`, duração 1400 → `op {0,90}`, `ed {1300,1400}`.
    - Títulos `OP`/`ED` em minúsculas → reconhecidos.
    - `[]` → `{op: null, ed: null}`.
    - `[{Chapter 1,0},{Chapter 2,600}]` → ambos `null`.
  - `episodeNumber`:
    - `"[FênixFansub_&_IsekaiSubs]_Fairy_Gone_01_[HDTV][1080p][8bits][877F5B35].mp4"` → 1
    - `"[Erai-raws] Fairy Tail - 100 Years Quest - 06 [1080p][HEVC][Multiple Subtitle][F52256A9].mkv"` → 6
    - `"Zombie.Detective.S01E03.1080p.NF.WEB-DL.DDP2.0.x264-HBO.mkv"` → 3
    - `"Naruto Ep. 012.mkv"` → 12
    - `"[Grupo] Filme [1080p].mkv"` → null
    - `"[Grupo] Titulo - 1080p.mkv"` → null (não confundir resolução com episódio)
  - `nextEpisode`, com os arquivos `["Ep 10.mkv","Ep 2.mkv","Ep 1.mkv","Ep 1.ass","notas.txt", pasta "Extras"]`:
    - a partir de `"Ep 1.mkv"` → `"Ep 2.mkv"`;
    - a partir de `"Ep 2.mkv"` → `"Ep 10.mkv"`;
    - a partir de `"Ep 10.mkv"` → `null`.
  - `isWatched`:
    - `(1200,1400,null)` → false
    - `(1260,1400,null)` → true (90%)
    - `(1291,1400,{1290,1380})` → true
  - `nextCardStart`: `(1400,{1290,1380},5)` → 1290; `(1400,null,10)` → 1390.
  - `pickTracks`, com as faixas sub `{1,"eng","English"}`, sub `{2,"por","Português (Brasil)"}`, sub `{3,"por","Signs"}`, audio `{1,"jpn"}` e audio `{2,"por"}`:
    - sem pref, defaults `por`/`jpn` → `{sid:2, aid:1}` (com o mesmo idioma, preferir a faixa cujo título não contém `sign`/`song`/`forced`);
    - pref `{subOff:true}` → `sid: "no"`;
    - pref `{subLang:"eng", subTitle:"English"}` → `sid: 1`;
    - pref com `subLang` inexistente → cai no padrão: `sid: 2`.
  - `formatTime`: 75 → `"1:15"`; 3725 → `"1:02:05"`; 0 → `"0:00"`.
- [ ] **Step 2: Rodar e ver falhar.** `bun run test` → falha: `logic.ts` não existe.
- [ ] **Step 3: Implementar `logic.ts`.**
  - **Títulos de abertura:** `/^(op|opening|intro|abertura)\b/i`.
  - **Títulos de encerramento:** `/^(ed|ending|outro|credits|encerramento)\b/i`.
  - **Intervalos:** `end` = `time` do capítulo seguinte, ou `duration`.
  - **`episodeNumber`, nesta ordem:**
    1. `S\d+E(\d+)`;
    2. `parseEpisode(name).number`;
    3. `[_\s.]-?[_\s.]?(\d{1,4})[_\s.]*(?=\[|$)`, desde que o número não seja seguido de `p` (resolução).
- [ ] **Step 4: Rodar e ver passar.** `bun run test` → todos passam. `bunx tsc --noEmit` sem erros.
- [ ] **Step 5: Commit.** `feat: lógica pura do player (capítulos, próximo episódio, faixas)`

---

### Task 5: Sessão e store do player no frontend

**Files:**
- Create: `src/lib/player/api.ts`, `src/lib/player/store.ts`, `src/lib/player/session.ts`

**Interfaces:**
- Consumes: os comandos da Task 2 e da Task 3, os tipos da Task 4.
- Produces, em `api.ts`: um objeto `playerApi` com `streamUrl(path, size)`, `data()`, `saveProgress(path, p)`, `saveTracks(release, pref)`, `saveSettings(s)` e `aniskip(malId, ep, duration)`. Os tipos espelham os do Rust em camelCase.
- Produces, em `store.ts`:
  ```ts
  export const playerStore: {
    ready: () => boolean;
    progress: (path: string) => Progress | undefined;
    settings: () => Settings;
    tracks: (release: string) => TrackPref | undefined;
    setProgress(path: string, p: Progress): void;   // atualiza a store na hora; grava com debounce de 5 s por path e imediatamente em flush()
    setTracks(release: string, pref: TrackPref): void;
    setSettings(s: Settings): void;
    flush(): Promise<void>;
  };
  ```
  Usar `createStore` de `solid-js/store` e carregar `playerApi.data()` uma vez (padrão parecido com `src/lib/catalog.ts`).
- Produces, em `session.ts`:
  ```ts
  export interface PlayTarget { dir: string; file: FileEntry; files: FileEntry[]; titlePath: string; malId: number | null; isAnime: boolean; animeTitle: string }
  export const player: { current: () => PlayTarget | null; open(t: PlayTarget): void; close(): void; switchTo(file: FileEntry): void };
  ```
  `release` = `dir` (a pasta do release). `isAnime` = o caminho começa com `Animes/`.

- [ ] **Step 1: Implementar os três arquivos.** Sem testes unitários: são finos sobre `invoke`, e a lógica está na Task 4.
- [ ] **Step 2: Verificar.** `bunx tsc --noEmit` e `bun run test` sem erros.
- [ ] **Step 3: Commit.** `feat: estado do player no frontend`

---

### Task 6: PlayerView — reprodução, barra, controles e atalhos

**Files:**
- Create: `src/views/PlayerView.tsx`, `src/components/player/SeekBar.tsx`, `src/components/player/Controls.tsx`
- Modify: `src/App.tsx`

**Interfaces:**
- Consumes: `createMpv` (Task 1), `player`/`playerStore`/`playerApi` (Task 5), `formatTime`/`isWatched` (Task 4).
- Produces:
  ```tsx
  export function PlayerView(props: { target: PlayTarget; onClose: () => void }): JSX.Element
  export function SeekBar(props: { position: number; duration: number; buffered: number; chapters: number[]; onSeek(sec: number): void }): JSX.Element
  export function Controls(props: { paused: boolean; volume: number; muted: boolean; position: number; duration: number; title: string; hasNext: boolean; fullscreen: boolean; on: ControlsHandlers }): JSX.Element
  ```
  `ControlsHandlers` = `{ togglePause, next, setVolume, toggleMute, openTracks, openSettings, openEpisodes, toggleFullscreen }`.

- [ ] **Step 1: `App.tsx`.** Quando `player.current()` não é nulo, renderizar `<PlayerView>` cobrindo a janela inteira (`fixed inset-0 z-40`, sem `bg`, para o vídeo aparecer). Nesse estado, esconder `TitleBar`, `Sidebar` e `main` com `invisible`, sem desmontar, para a página da obra manter o scroll ao voltar.
- [ ] **Step 2: `PlayerView`, carregamento.**
  - Ao montar: `url = await playerApi.streamUrl(path, size)`, depois `createMpv` com `demuxer-max-bytes = ${settings.cacheMiB}MiB`, depois `load(url, progresso salvo se não visto)`.
  - `onCleanup` faz `saveProgress`, `flush` e `destroy`.
  - Trocar de episódio (`switchTo`) faz `load` no mesmo mpv.
- [ ] **Step 3: Barra de título sobreposta.** Botão ← (`aria-label="Voltar"`, chama `onClose`), `animeTitle · Ep. N` e os botões de janela.
  - Reaproveitar os botões do `TitleBar.tsx`. Se forem internos, extrair `WindowButtons` para um export.
  - Área de arraste com `data-tauri-drag-region`.
- [ ] **Step 4: `SeekBar`.**
  - Trilha com três camadas: carregado (`(position + demuxer-cache-time) / duration`, branco 32%), progresso (`--accent`) e marcas de capítulo (traço de 2 px).
  - Clique e arraste com `pointer events`. O seek só é enviado no `pointerup`; durante o arraste, só a interface acompanha.
  - Ao passar o mouse, tooltip com `formatTime`.
  - `role="slider"`, com `aria-valuenow`/`aria-valuemax` e `aria-valuetext` = `formatTime`.
- [ ] **Step 5: `Controls`.** Linha do mockup A: play/pause, próximo, volume (slider), tempo, título no centro, CC, ⚙, ☰ e tela cheia. Botões só com ícone levam `aria-label`. Alvos de clique ≥ 40 px.
- [ ] **Step 6: Auto-hide e atalhos.**
  - Os controles somem após 2500 ms sem mexer o mouse, só com o vídeo tocando. O cursor também some (`cursor-none`).
  - Atalhos da seção Global Constraints em `keydown` no `window`, ignorando quando o foco está em `input`, `select` ou num menu aberto.
  - Duplo clique no vídeo alterna tela cheia.
- [ ] **Step 7: Carregando e erros.**
  - `paused-for-cache` verdadeiro mostra um spinner discreto no centro (`LoaderCircle animate-spin`).
  - Evento de fim com `reason === "error"` mostra o aviso `Sem conexão com a nuvem. Tentando de novo…` e recarrega na mesma posição com espera crescente (2 s, 4 s, 8 s, até 30 s).
- [ ] **Step 8: Progresso.**
  - A cada `time-pos`, `playerStore.setProgress(path, {position, duration, watched: isWatched(...), updatedAt})`. A store já faz o debounce.
  - Ao pausar e ao fechar, `flush()`.
- [ ] **Step 9: Verificar à mão** com `bun tauri dev`:
  - um `.mp4` (Fairy Gone), um `.mkv` HEVC (Fairy Tail 100 Years Quest) e um dorama `.mkv` de mais de 2 GB (Zombie Detective);
  - cada um começa a tocar em menos de 10 s;
  - um seek para 80% retoma em menos de 10 s;
  - a legenda `.ass` aparece estilizada;
  - durante a reprodução, a pasta Downloads e o `%TEMP%` não crescem (conferir o tamanho antes e depois);
  - `bunx tsc --noEmit` sem erros.
- [ ] **Step 10: Commit.** `feat: tela do player com controles, barra e atalhos`

---

### Task 7: Pular abertura, próximo episódio, faixas, configurações e episódios

**Files:**
- Create: `src/components/player/SkipButton.tsx`, `NextCard.tsx`, `TracksMenu.tsx`, `SettingsMenu.tsx`, `EpisodesPanel.tsx`
- Modify: `src/views/PlayerView.tsx`

**Interfaces:**
- Consumes: `classifyChapters`, `episodeNumber`, `nextEpisode`, `videoFiles`, `nextCardStart`, `pickTracks` (Task 4); `playerApi.aniskip` e `playerStore` (Task 5).

- [ ] **Step 1: Tempos de abertura e encerramento.** Quando `chapter-list` e `duration` chegam: `skip = classifyChapters(...)`. Se `op` e `ed` forem nulos e `target.isAnime`, `malId` e `episodeNumber(file.name)` existirem, `skip = await playerApi.aniskip(malId, ep, duration)`. Uma vez por arquivo.
- [ ] **Step 2: `SkipButton`.**
  - Visível quando `settings.showSkipIntro && op && position ∈ [op.start, op.end - 1)`.
  - Clicar ou apertar `S` faz `set("time-pos", op.end)`.
  - Fica no canto inferior direito, acima dos controles, e continua visível mesmo com os controles escondidos.
  - Texto exato: `Pular abertura`.
- [ ] **Step 3: `NextCard`.**
  - Com `next = nextEpisode(target.files, file.name)` não nulo e `position ≥ nextCardStart(duration, ed, settings.countdownSec)` (e não cancelado para este arquivo), aparece o cartão.
  - Conteúdo: `Próximo episódio em {n}`, com contagem a partir de `countdownSec`; o nome do próximo episódio (`Ep. N` ou o nome do arquivo); botões `Assistir agora` e `Cancelar`.
  - O botão primário tem uma barra de preenchimento animada.
  - Com `settings.autoNext`, o fim da contagem faz `player.switchTo(next)`. Sem ele, o cartão aparece sem contagem: só o botão `Assistir agora`, e o título vira `Próximo episódio`.
  - `Cancelar` esconde o cartão até o fim do arquivo.
  - Com `eof-reached` e `autoNext` sem cancelamento, vai para o próximo na hora.
  - `N` vai para o próximo em qualquer momento.
- [ ] **Step 4: `TracksMenu`.**
  - Seções `Legendas` (com `Desligada`) e `Áudio`, a partir do `track-list`, mostrando o título ou o idioma.
  - Ao abrir um arquivo: `pickTracks(tracks, playerStore.tracks(dir), settings)` e aplicar `sid`/`aid`.
  - Escolha manual: `playerStore.setTracks(dir, pref)` com idioma e título da faixa escolhida.
- [ ] **Step 5: `SettingsMenu`.**
  - Os cinco controles da spec, com os rótulos exatos da seção Global Constraints. Cada alteração faz `playerStore.setSettings`.
  - `Memória à frente` vale a partir do próximo arquivo: aplicar também agora com `set("demuxer-max-bytes", …)`.
- [ ] **Step 6: `EpisodesPanel`.** Painel lateral sobreposto (`☰`) com `videoFiles(target.files)`: número, rótulo, barrinha do progresso salvo e destaque no atual. Clicar faz `switchTo`. Fecha com `Esc` ou ao clicar fora.
- [ ] **Step 7: Verificar à mão:**
  - Fairy Tail 100 Years Quest (tem capítulos): o botão aparece e pula; o cartão aparece no ED; a contagem leva ao ep. 2.
  - Fairy Gone `.mp4` (sem capítulos): consulta o AniSkip (a requisição aparece no log) e, rodando de novo, usa o cache sem requisição.
  - Um dorama: não consulta o AniSkip, e o cartão aparece em `duration - countdown`.
  - A configuração `Ir para o próximo episódio sozinho` desligada não avança.
  - As faixas escolhidas são lembradas ao abrir o ep. 2.
  - `bunx tsc --noEmit` sem erros.
- [ ] **Step 8: Commit.** `feat: pular abertura, próximo episódio, faixas e configurações no player`

---

### Task 8: Entradas pela página da obra, continuar e documentação

**Files:**
- Modify: `src/components/FileList.tsx`, `src/views/TitleView.tsx`, `README.md`
- Variante B (só se a Task 1 decidiu pelo Plano B): `src-tauri/src/player/commands.rs` (`player_open_window`), `src-tauri/capabilities/default.json` (janela `player`), `src/index.tsx` (renderiza só o `PlayerView` quando `getCurrentWindow().label === "player"`)

**Interfaces:**
- Consumes: `player.open(PlayTarget)` (Task 5), `playerStore.progress` (Task 5), `videoFiles`/`episodeNumber`/`formatTime` (Task 4).
- `FileList` ganha a prop `onPlay?: (file: FileEntry) => void`.

- [ ] **Step 1: `FileList`.**
  - Para arquivos com extensão em `VIDEO_EXT` e com `onPlay` definido, botão primário `Assistir` (ícone `Play`) antes do `DownloadButton`.
  - Abaixo do rótulo, uma barrinha de progresso (`h-0.5 bg-accent`) quando há progresso salvo, e um ícone `Check` com o texto `Visto` (para leitores de tela) quando `watched`.
- [ ] **Step 2: `TitleView`.**
  - Passar `onPlay` montando o `PlayTarget` com `dir = filesPath()`, `files = filesListing().files`, `titlePath`, `malId = media()?.idMal ?? null`, `isAnime` e `animeTitle`.
  - No herói, quando há algum progresso não visto (ou o próximo depois do último visto) no release selecionado, o botão primário vira `Continuar ep. {n} ({mm:ss})` e o `Baixar todos` passa a ser secundário.
- [ ] **Step 3: Variante B (se aplicável).**
  - `player_open_window` cria ou foca a janela `player` (`WebviewWindowBuilder`, `transparent(true)`, `decorations(false)`, 1280×720) e emite o `PlayTarget` para ela.
  - O ← fecha a janela e foca a `main`.
  - O progresso é recarregado na `main` quando a janela `player` fecha (evento `player://closed`, que dispara `playerStore` recarregar).
- [ ] **Step 4: README.**
  - Em "O que faz", adicionar o player (streaming sem baixar, pular abertura, próximo episódio, continuar).
  - Em "Requisitos", como obter a libmpv (passo do plugin).
  - Créditos: mpv (licença LGPL/GPL, conforme a build usada) e AniSkip.
- [ ] **Step 5: Verificação final.**
  - `cargo clippy` e `cargo test` sem erros; `bun run test` e `bunx tsc --noEmit` sem erros; `bun run build` ok.
  - Ponta a ponta: abrir a obra → `Assistir` → assistir 1 min → voltar → a barrinha aparece e o botão `Continuar ep. 1 (1:00)` funciona.
- [ ] **Step 6: Commit.** `feat: assistir e continuar pela página da obra`

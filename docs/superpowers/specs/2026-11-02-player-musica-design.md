# Player de músicas (OSTs): design

Data: 2026-11-02. Escopo: só o player de músicas. Branch `feat/player-musica`, criada a partir de `feat/leitor-manga` (que ainda não foi para o master e mexe nos mesmos arquivos, como `FolderView`, `FileList` e `App.tsx`).

## Intenção

A seção OSTs do acervo hoje só serve para baixar. O dono quer **abrir um álbum e dar play**, com a música **na melhor qualidade possível**, tocando direto da nuvem, sem baixar antes. A referência de uso é o Spotify: barra embaixo que continua tocando enquanto se navega, fila, aleatório e repetir. A referência visual é o leitor de mangá do próprio app: um **encarte de CD japonês**.

O dono não ouve muita OST hoje. O sucesso é o player ser agradável o bastante para isso mudar, e o áudio nunca ficar pior do que o arquivo.

## Fatos verificados (sondagem de 2026-11-02, ~500 faixas em 57 pastas)

- **Estrutura:** `OSTs/{Animes, Doramas, Filmes Japoneses, Jogos, Tokusatsu}/<Letra>/<Título>/<Álbum>/faixas`. Às vezes as faixas estão direto na pasta do título.
  - O nome da pasta do álbum costuma trazer data e código do disco, por exemplo `2011.05.25 - A-Channel TV1 OP1 - Morning Arch (SVWC-7767)` ou `[Album] V.A. - Studio Ghibli no Uta [2008.11.26][APE]`.
- **Faixas soltas**, uma por arquivo, sem zip:
  - FLAC 261 (média de 20 MB, até ~40 MB);
  - MP3 236 (~5 MB);
  - WAV 14 (~42 MB).
  - Raro: **APE + CUE**, um arquivo por disco (~300 MB) com `.cue` marcando as faixas.
  - Os nomes de arquivo são do tipo `01 - Título.flac` ou `1-01 - Título.flac` (disco-faixa). Há nomes em japonês.
- **Imagens:** `.jpg` soltos ou numa subpasta (scans) dentro do álbum.
- **Range** funciona na nuvem: `GET /api/preview?path=` com `Range` responde 206. O leitor de mangá já usa isso via `AnitsuHttp::send_raw` (`src-tauri/src/reader/mod.rs`, `fetch_range`).
- **Limite de requisições:** a nuvem respondeu **HTTP 429 "Too many requests"** depois de ~57 listagens seguidas. Isso vale para qualquer coisa que faça muitas requisições em sequência.

## Decisões do dono

1. **Escopo:** abrir um álbum e dar play basta. Sem favoritos, playlists, histórico nem biblioteca global.
2. **Qualidade:** "a melhor qualidade da música". O áudio é decodificado e tocado pelo **Rust**, não pelo `<audio>` do webview. O dono pediu uma crate Rust que melhore a música; a leitura acordada é **fidelidade, não efeito**:
   - nada de equalizador nem "som melhorado";
   - **volume igualado entre faixas**, ligado por padrão e com opção de desligar.
3. **Layout B, "Encarte de CD japonês"** (escolhido no mockup):
   - coluna da esquerda com a capa grande;
   - sobre a borda direita da capa, uma **faixa vermelha vertical (o "obi")** com o código do disco em texto vertical, na fonte mincho;
   - abaixo da capa, o nome do álbum em mincho, a linha da obra e do ano, e o botão "Tocar";
   - à direita, a lista de faixas com **números grandes em mincho** (`01`, `02`…), título e duração. A faixa tocando fica com o número e o título em `--color-brand-ink` e o fundo levemente avermelhado;
   - embaixo da lista, a linha de formato, por exemplo "FLAC · 16 bit / 44,1 kHz · sem perdas";
   - **barra fixa embaixo** do app, que continua tocando ao navegar: play/pausa, nome da faixa e da obra, tempo, progresso no **"fio" vermelho** (o mesmo do leitor: linha fina, trecho tocado em vermelho e um traço marcando a posição) e os botões anterior, próxima, aleatório, repetir e volume;
   - mesma família visual do leitor (`--font-mincho`, `--color-brand`, retícula).
4. **Funcionamento** (proposto e aprovado; os itens marcados "(do Claude)" foram decisão do Claude, aprovada junto):
   - Uma pasta com faixas de áudio (`.flac .mp3 .wav .ogg .m4a .opus`) abre como **página de álbum**. Subpastas e os outros arquivos continuam acessíveis numa aba "Arquivos" (do Claude).
     - **Onde:** em qualquer pasta, não só em `OSTs/` (do Claude). Pastas de anime ou mangá com extras de áudio também ganham o encarte. Isso vale tanto pelo `FolderView` quanto pelo `TitleView` (ver `classify` em `src/lib/library.ts`). Numa página de obra (`TitleView`), as faixas aparecem como uma seção "Músicas" com play, e "Abrir encarte" leva à página do álbum.
     - **Discos em subpastas** (`Disc 1/`, `CD2/`, `Disco 2`, sem diferenciar maiúsculas): a pasta-pai é o álbum. A fila percorre os discos em ordem, com separador "Disco N". Isso custa uma listagem a mais por disco, feitas uma por vez (do Claude).
   - **Capa:** imagem da pasta ou de subpasta; dar preferência a nomes com `cover`, `front` ou `folder`; senão, a primeira imagem; senão, a capa embutida na própria faixa (do Claude).
   - **Obi:** código do disco extraído do nome da pasta, entre parênteses ou colchetes, no padrão `ABCD-1234`. Sem código, mostra o ano (do Claude).
   - **Título e duração** vêm das tags quando existem; senão, do nome do arquivo sem o número. A duração **aparece aos poucos**, conforme o app lê o começo de cada arquivo (do Claude).
   - **Discos:** faixas `1-01`, `2-01` ganham um separador "Disco 1", "Disco 2".
   - **Fila = o álbum.** Aleatório; repetir álbum, repetir faixa ou não repetir.
   - **Sem pausa entre faixas (gapless):** a próxima faixa é preparada antes de a atual acabar (do Claude). Muitas OSTs emendam faixas.
   - **Teclas de mídia** do teclado e o controle de mídia do Windows (do Claude).
   - O app lembra o **volume** e **onde parou em cada álbum** (faixa e posição); a página do álbum oferece "Continuar" (do Claude).
5. **Fora por enquanto:**
   - APE+CUE e outros formatos que o decodificador não lê aparecem na lista com só "Baixar";
   - favoritos, playlists, histórico;
   - letras;
   - equalizador.

## Não-objetivos

- Biblioteca global ("todas as músicas", busca por faixa). O limite de 429 torna inviável varrer o acervo.
- Tocar vídeo, ou música dentro do player de vídeo.
- Linux e macOS. O alvo é Windows; não quebrar o build nos outros é desejável.

## Anti-objetivos (seria falha mesmo funcionando)

- **Piorar o áudio:**
  - reamostrar com qualidade baixa;
  - aplicar ganho que mude dentro de uma faixa, como um compressor ou "AGC" audível;
  - cortar o sinal (clipping) ao igualar o volume;
  - reduzir FLAC para 16 bit sem necessidade.
- **Sobrecarregar a nuvem** e tomar 429:
  - nada de ler o cabeçalho de 75 faixas em paralelo ao abrir um álbum;
  - nada de pré-carregar o álbum inteiro.
- **Expor sessão ao JS**, como em todo o projeto: os tokens ficam no Rust.
- **Música parar ao navegar** pelo app, ou ao abrir o leitor de mangá. Ao abrir o player de vídeo, a música **pausa** (do Claude).
- Esperar o download da faixa inteira para começar a tocar.
- Visual genérico de "player de música". A identidade é o encarte.

## Arquitetura proposta

```
SolidJS                                    Rust (Tauri)
 AlbumView (encarte)  ─ invoke ─▶  music::commands
 MusicBar (fixa)      ◀─ eventos ─  music::engine (thread de áudio)
                                     ├─ fonte: nuvem por Range (AnitsuHttp, como reader::fetch_range)
                                     ├─ decodificação: symphonia
                                     ├─ reamostragem: rubato (só quando necessário)
                                     ├─ volume igualado: ReplayGain das tags ou ebur128
                                     └─ saída: cpal (WASAPI)
```

### Fonte por pedaços

- Um `MediaSource` (symphonia) próprio que lê a faixa da nuvem com buffer em RAM e `seek`. O áudio não é gravado em disco.
- **Um pedido por faixa (decisão do Claude):** um único `Range: bytes=N-` aberto, lido em stream conforme chega. Não se sabe se o 429 também conta os pedidos de `/preview`, então o menor número de pedidos é o caminho seguro. Uma faixa de 40 MB custa 1 pedido.
- A faixa atual e a próxima ficam inteiras em RAM, à medida que chegam (no máximo ~80 MB).
- Seek para um ponto ainda não recebido abre um novo `Range` a partir dali; o que já chegou fica.
- Ao receber 429, esperar (respeitando `Retry-After`, se vier) e tentar de novo, sem derrubar a reprodução do que já está em buffer.

### Decodificação e saída

- `symphonia` com os formatos FLAC, MP3, WAV/PCM, Vorbis, AAC/M4A e ALAC. Decodificar em `f32`.
- `cpal` em modo compartilhado do WASAPI. Se a taxa da faixa diferir da do dispositivo, reamostrar com `rubato`, com um reamostrador sinc de alta qualidade. Quando as taxas coincidem, não passar pelo reamostrador.
- O volume do app é aplicado em `f32` antes da saída, junto com o ganho de normalização.
- **Gapless:** a próxima faixa é aberta e decodificada um pouco antes do fim da atual; as amostras são emendadas no mesmo fluxo de saída. Se as taxas forem diferentes, recriar o fluxo é aceitável.

### Volume igualado

- O ganho é **fixo por faixa**, decidido antes de ela tocar, e nunca muda no meio.
- **Ganho de álbum, não de faixa (decisão do Claude).** OST tem faixas baixas de propósito (ambiente, interlúdio). O ganho por faixa achataria isso; o de álbum preserva a diferença entre faixas e só iguala um álbum com outro.
- Ordem das fontes:
  1. tag ReplayGain de **álbum**;
  2. sem ela, tag ReplayGain de **faixa**;
  3. sem tags: medição EBU R128 com `ebur128` das faixas que já tocaram, guardada em cache em disco (`app_data_dir`; só números, nunca áudio). O ganho do álbum é a média de loudness das faixas medidas daquele álbum. Medir só o que já se toca, nunca baixar faixas só para medir.
  4. Álbum sem tag e sem nenhuma faixa medida: 0 dB.
  - Consequência aceita: num álbum sem tags, o ganho se ajusta faixa a faixa nas primeiras audições e estabiliza.
- Alvo de -18 LUFS, mais ou menos o nível do ReplayGain (do Claude). Limitar o ganho pelo pico para não cortar o sinal.

### Metadados da página do álbum

- Ao abrir o álbum, a lista aparece na hora, com os nomes vindos dos arquivos.
- Tags e duração são lidas em segundo plano, **uma faixa por vez**, com só os primeiros KB de cada arquivo (cabeçalho FLAC/ID3/MP4). Ao receber 429, esperar e continuar.
- Guardar em cache no disco, como metadados por caminho e tamanho.
- A capa embutida é lida só se não houver imagem na pasta.

### Estado e eventos

- O Rust emite `music://state` (faixa, posição, duração, tocando/pausado, buffer carregado, formato) algumas vezes por segundo. O JS não controla o tempo; só envia comandos: `play(album, index)`, `pause`, `seek`, `next`, `prev`, `set_volume`, `set_shuffle`, `set_repeat`, `set_normalize`.
- As teclas de mídia e o controle do Windows usam a SMTC (`windows` crate) ou `souvlaki`.
- Progresso por álbum e preferências ficam no `localStorage`, como no leitor: chave `higanbana:music:v1`.

## Fica a cargo do construtor

- Tamanho dos blocos e da leitura antecipada.
- Se o motor roda numa thread própria ou numa tarefa tokio com uma thread de saída do cpal.
- Como emendar o gapless quando as taxas diferem.
- A escolha entre `souvlaki` e a SMTC direta.
- Detalhes visuais finos da página e da barra, dentro do layout B. Seguir o mockup aprovado em `.superpowers/brainstorm/1445-1793599640/content/player-layout.html` (opção B).
- Atalhos de teclado quando a página do álbum tem foco: espaço para pausar, ← → ±5 s (do Claude).

# Player de vídeo (anime e dorama): design

Data: 2026-11-01. Escopo: só o player de vídeo. O reader de mangá e o player de música são projetos separados, para depois.

## Intenção

Hoje, para assistir algo do acervo, é preciso baixar o episódio inteiro. O dono do projeto **não quer ocupar espaço em disco** (há pouco espaço livre), e muitos arquivos são enormes: doramas `.mkv` de 1,2 a 2,7 GB por episódio. O objetivo é **assistir direto da nuvem, em streaming**, como o Stremio faz: o app baixa só os pedaços perto do ponto em que a pessoa está, permite pular para qualquer lugar na hora e não deixa nada no disco no fim.

O player deve ser bom para **maratonar**: pular abertura, ir sozinho para o próximo episódio e retomar de onde parou, com uma interface que parece parte do app, não um player genérico.

## Fatos verificados (sondagem de 2026-11-01)

- **Range funciona.** Na nuvem, `GET /api/download?path=` e `GET /api/preview?path=` respondem `206 Partial Content` com `Accept-Ranges: bytes` e `Content-Range: bytes a-b/total`. Um Range no meio do arquivo (100 MiB) também funcionou.
  - O `preview` devolve `Content-Type` real (`video/mp4`).
  - O `download` devolve `application/octet-stream`.
- **Autenticação.** A nuvem autentica pelo cookie de sessão. O app já monta esse cookie no Rust (`AuthManager::cookie_header`, `AnitsuHttp::send`, com renovação automática em 401). O access token expira em cerca de 1 h.
- **Formatos encontrados no acervo:**
  - Anime de fansub antigo: `.mp4` H.264 8-bit, cerca de 500 MB.
  - Anime de releases novos (Erai-raws etc.): `.mkv` HEVC, várias legendas (inclusive `.ass` estilizada).
  - Dorama: `.mkv` x264 com áudio DD+ (E-AC-3), de 1,2 a 2,7 GB.
  - O WebView2 não toca de forma confiável MKV, HEVC, DD+ nem `.ass`. Por isso o motor é o mpv.
- **Listagem.** `GET /api/files?path=` traz `name, extension, size, modified, is_directory`. Não há miniaturas por episódio.
- **AniList.** As queries já buscam `idMal`, que é necessário para o AniSkip.
- **Janela do app.** Ela tem `decorations: false` (barra de título própria) e hoje não é transparente.

## Decisões do dono

1. **Motor:** o mpv embutido (libmpv), com **interface 100% própria** no estilo do app: tema "Cinema" escuro, accent laranja `--accent`, fontes Outfit/Inter, ícones lucide-solid, `--ease-expo`. Nada da OSC/OSD do mpv aparece.
2. **Streaming por pedaços, sem disco.** Nada do vídeo é gravado em disco; o cache de leitura antecipada fica na RAM.
3. **Layout A, "Imersivo"** (aprovado no mockup):
   - o vídeo ocupa a janela inteira;
   - a barra de título (← voltar, nome da obra, número do episódio, botões de janela) e os controles inferiores ficam sobre o vídeo, com gradiente, e **somem com o mouse parado**;
   - a barra de progresso mostra três camadas: progresso em accent, trecho já carregado na memória em branco translúcido e marcas de capítulo;
   - linha de controles: play/pause, próximo episódio, volume, tempo, título do episódio no centro, legendas/áudio (CC), configurações (⚙), lista de episódios (☰, abre um painel lateral sobreposto) e tela cheia.
4. **Pular abertura.** Um botão "Pular abertura" aparece no canto inferior direito durante a abertura. Clicar nele, ou apertar `S`, salta para o fim da abertura. O botão some sozinho quando a abertura termina.
5. **Próximo episódio.** Quando o encerramento começa, o vídeo **continua tocando** e aparece um cartão "Próximo episódio em N", com contagem regressiva visível, botões "Assistir agora" e "Cancelar", e o nome do próximo episódio.
   - Se a pessoa não cancelar, abre o próximo episódio.
   - Se cancelar, o encerramento segue normalmente.
   - O próximo episódio é o arquivo de vídeo seguinte na mesma pasta, em ordem natural.
   - No último episódio da pasta não aparece cartão.
   - **Como na Netflix: o cartão aparece no início dos créditos.**
   - **Sem tempo de encerramento conhecido** (decisão do dono, 2026-11-01):
     - em anime de 20–30 min, os créditos são estimados em ~1:30 antes do fim;
     - fora disso, o cartão aparece nos últimos segundos do arquivo, iguais à duração da contagem.
6. **Fontes dos tempos de abertura e encerramento**, em ordem:
   1. **capítulos do arquivo** (MKV);
   2. se não houver, a **API do AniSkip** como reserva;
   3. sem nenhum dos dois: não aparece botão de pular, e o próximo episódio segue o caso "sem tempo de encerramento conhecido" acima.
7. **Configurações** (no menu ⚙ do player e numa tela de configurações):
   - ir para o próximo episódio sozinho (liga/desliga, **pedido explícito do dono**);
   - duração da contagem: 5, 10 ou 15 s, padrão 5;
   - mostrar o botão "Pular abertura" (liga/desliga);
   - idioma preferido de legenda e de áudio;
   - tamanho da memória de leitura antecipada.
8. **Onde abre.** Na mesma janela, como uma rota/tela do app. Um botão "Assistir" (ícone Play) em cada episódio da página da obra abre o player. ← volta para a página da obra. Duplo clique ou `F` alterna tela cheia.
9. **Continuar de onde parou.**
   - O app guarda localmente a posição de cada episódio.
   - A página da obra mostra uma barra de progresso por episódio e um botão principal "Continuar ep. N (mm:ss)".
   - Um episódio conta como visto a partir de 90% ou quando o encerramento começa.
10. **Faixas.** O menu CC lista as faixas de legenda (incluindo "desligada") e de áudio do arquivo. A escolha fica **lembrada por obra**.
11. **Atalhos:**

    | Tecla | Ação |
    |---|---|
    | Espaço | pausa |
    | ← / → | ±5 s |
    | ↑ / ↓ | volume |
    | M | mudo |
    | F | tela cheia |
    | S | pular abertura |
    | N | próximo episódio |
    | Esc | sai da tela cheia |

12. **Carregando e erros.** Enquanto o pedaço necessário não chega, aparece um indicador discreto no centro. Se a rede cair, o player mostra um aviso e tenta de novo sozinho.

## Não-objetivos (por enquanto)

- Assistir junto, Chromecast, picture-in-picture.
- Baixar para ver offline pelo player. O download comum que já existe continua igual.
- Reader de mangá e player de música (projetos seguintes).
- Linux e macOS. O alvo é Windows, onde o app é usado. Não quebrar o build nos outros sistemas é desejável, mas o player pode ficar indisponível neles.

## Anti-objetivos (seria falha mesmo funcionando)

- **Gravar o vídeo em disco**, mesmo temporariamente, ou baixar o arquivo inteiro antes de tocar.
- **Sobrecarregar as APIs** (preocupação recorrente do dono):
  - nada de pedir o arquivo inteiro de uma vez nem fazer prefetch agressivo;
  - o AniSkip é consultado só quando o arquivo não tem capítulos, uma vez por episódio, com cache em disco.
- Expor tokens de sessão ao JavaScript ou em argumentos de linha de comando do mpv. **Os tokens ficam no Rust**, regra que já vale no projeto.
- Mostrar qualquer interface do próprio mpv, ou um visual que destoe do app.
- Ser preciso esperar o "carregamento" de um arquivo de GB antes de começar a ver.

## Arquitetura proposta

```
SolidJS (PlayerView)  ── invoke/eventos ──▶  Rust (Tauri)
   controles, overlays                         ├─ mpv (libmpv), desenhando atrás do webview transparente
                                               ├─ proxy de stream local (127.0.0.1, porta aleatória)
                                               │     mpv ──HTTP Range──▶ proxy ──Range + cookie──▶ nuvem
                                               ├─ skip: capítulos do mpv → AniSkip (cache em disco)
                                               └─ progresso assistido / preferências (JSON no app_data_dir)
```

### Proxy de stream (o "motor de pedaços")

- Servidor HTTP mínimo no Rust, só em `127.0.0.1`, numa porta aleatória, com um token aleatório por sessão no caminho, para que outros processos da máquina não usem o proxy.
- O mpv abre `http://127.0.0.1:<porta>/<token>/stream?path=<caminho na nuvem>`.
- O proxy repassa o `Range` do mpv para a nuvem usando o `AnitsuHttp` da nuvem (cookie e renovação em 401 já existentes). Ele devolve status, `Content-Range`, `Content-Length`, `Accept-Ranges` e `Content-Type`, e faz stream do corpo sem bufferizar o arquivo.
- Motivo: o token expira em cerca de 1 h, e um episódio longo com seeks precisaria de cookie novo no meio. Passar o cookie direto ao mpv (`http-header-fields`) quebraria isso e exporia o token.
- Cache do mpv só em RAM:
  - `cache=yes` e `cache-on-disk=no`;
  - `demuxer-max-bytes` em torno de 150–300 MiB (configurável);
  - `demuxer-max-back-bytes` menor, para voltar alguns segundos sem pedir de novo.
- O mpv já pede por Range e faz seek por Range; não é preciso lógica de pedaços própria além do proxy.

### Embutir o mpv na janela

- **Abordagem:** o mpv desenha numa janela nativa atrás do webview, e o webview fica transparente enquanto o player está aberto. Os controles são HTML por cima.
- **Primeira opção:** o crate `tauri-plugin-libmpv` (0.3.x, com Windows testado). Ele exige janela `transparent` e fundo transparente no HTML/CSS e permite definir o `wid`.
- **Se o plugin não servir** (maturidade, controle de propriedades e eventos, conflito com a barra de título própria): integrar a libmpv direto (por exemplo, o crate `libmpv2`) com `wid` de uma janela-filha.
- **Resultado do spike (2026-11-01): mesma janela.** tauri-plugin-libmpv 0.3.2 + libmpv LGPL (zhongfly). Vídeo atrás do webview transparente com HTML clicável por cima; maximizar, restaurar, tela cheia e voltar funcionaram (conferido por capturas de tela). Tremor durante o redimensionamento com arraste não foi medido de forma automática; o dono confere no uso.
- **Plano B (decisão do dono):** se o spike mostrar que embutir na janela principal fica ruim (tremor ao redimensionar, problemas em tela cheia, com a barra de título ou a sombra), o player abre numa **janela própria do Tauri**, só para ele, sem bordas, mantendo a interface 100% própria. Cede a regra "mesma janela", nunca a de "interface própria". Nesse caso o ← fecha a janela do player e foca a principal.
- **Janela transparente afeta o app todo.** As outras telas devem continuar com fundo opaco: o fundo passa do `html/body` para o contêiner de cada tela, e só a tela do player deixa o fundo vazio.
- **Distribuição:** a DLL da libmpv (cerca de 30–40 MB) vai junto no instalador como recurso do Tauri. Ela não entra no git; um script de setup a baixa, como o plugin já faz.

### Tempos de abertura e encerramento

- **Capítulos:** ler a propriedade `chapter-list` do mpv depois de carregar o arquivo. Classificar pelos títulos:
  - abertura: `op`, `opening`, `intro`, `abertura`;
  - encerramento: `ed`, `ending`, `outro`, `credits`, `encerramento`;
  - comparação sem diferenciar maiúsculas e tolerante a variações.
  - O fim de um capítulo é o início do seguinte.
- **AniSkip:** `GET https://api.aniskip.com/v2/skip-times/{malId}/{episódio}?types[]=op&types[]=ed&episodeLength={segundos}`.
  - `malId` vem do `idMal` do AniList da obra.
  - O número do episódio é extraído do nome do arquivo.
  - Cache em disco por `(malId, episódio)`, inclusive para "não encontrado", com um TTL longo.
  - Feito do lado Rust.
  - Sem `idMal` ou sem número de episódio: não consulta.
  - Doramas não têm entrada no AniSkip, então não consultar para eles.

### Estado e persistência

- Progresso por arquivo: chave é o caminho na nuvem; valor é posição, duração, visto e data.
- Preferências do player e faixas escolhidas por obra.
- Arquivo JSON no `app_data_dir`, como o `session.json` já é.

## Decisões tomadas na revisão (escolhas do Claude, não do dono; conferir)

- **Unidade de memória:**
  - O progresso é **por arquivo** (caminho na nuvem).
  - As faixas lembradas são **por pasta de release** (`Release [tags]`), guardando **idioma e título da faixa**, nunca o índice, porque o layout de faixas muda entre releases.
  - Se o idioma lembrado não existir no arquivo, usa o idioma preferido global.
  - O "Continuar ep. N" na página da obra aponta para o **último arquivo assistido** dentro da temporada e do release selecionados na página (a página já tem os chips de Temporada e Versão).
- **Episódio já baixado:** se o arquivo já existe na pasta de downloads do app, com tamanho igual ao da nuvem, o player abre o arquivo local em vez de fazer streaming. Isso evita pedidos à nuvem. A chave do progresso continua sendo o caminho na nuvem.
- **Carga na nuvem:**
  - O proxy usa `/api/preview`, o endpoint feito para ver no navegador, que devolve o `Content-Type` real.
  - Limite de **2 conexões simultâneas com a nuvem por vídeo**. Quando o mpv abandona uma conexão (por exemplo, num seek), a requisição de cima é cancelada na hora.
  - Leitura antecipada padrão: **150 MiB**.
- **Licença da libmpv:** o app é MIT e público no GitHub. Preferir uma build **LGPL** da libmpv (`-Dgpl=false`), carregada dinamicamente como DLL separada. Citar o mpv e a licença dele no README e no "Sobre". Se só houver build GPL prática, registrar isso no README.

## Deixado para quem for construir

- Escolher entre `tauri-plugin-libmpv` e integração direta com a libmpv, depois de um spike curto.
- Detalhes do proxy: crate de servidor HTTP, cancelamento quando o mpv fecha a conexão, como cada requisição de Range é feita.
- Regex de número de episódio a partir do nome do arquivo e ordem natural dos arquivos.
- Formato exato dos arquivos JSON e TTLs de cache.
- Microinterações: tempos de animação, auto-hide de cerca de 2,5 s, preview de tempo ao passar o mouse na barra (sem miniatura).
- Onde fica a "tela de configurações" global, se ainda não existir uma.
- Como lidar com arquivos que não são vídeo numa pasta de episódios (`.ass` soltos, `.txt` etc.): ignorar na lista de episódios.
